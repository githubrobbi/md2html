// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::default_numeric_fallback,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    clippy::min_ident_chars,
    clippy::mixed_read_write_in_expression,
    clippy::shadow_reuse,
    clippy::std_instead_of_core,
    clippy::string_slice,
    clippy::unseparated_literal_suffix,
    elided_lifetimes_in_paths,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

//! Copies of a page for reading elsewhere: a PDF, printed by the Chrome
//! already on the machine, and an EPUB for e-readers, written here.
//!
//! Both start from the finished page, so they say exactly what it says.
//! md2html takes on no dependency for either: the PDF needs a Chrome (or
//! Chromium) and says so plainly when there is none; the EPUB is XHTML,
//! a stylesheet and the page's images in a zip written with the standard
//! library.
//!
//! Exception: PDF and EPUB export with the XHTML and zip writers they need,
//! kept beside each other (`scripts/ci/file_size_exceptions.txt`).

use core::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::text::{base64_decode, esc, hex, sha256};

/// How long Chrome may take to print one page.
const PRINT_LIMIT: Duration = Duration::from_secs(90);

/// The Chrome to print with: `MD2HTML_CHROME`, else the usual places.
fn chrome() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MD2HTML_CHROME") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let fixed = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ];
    if let Some(found) = fixed.iter().map(PathBuf::from).find(|p| p.is_file()) {
        return Some(found);
    }
    let names = [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
    ];
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
        .find(|p| p.is_file())
}

/// A file as a `file:` URL.
fn file_url(path: &Path) -> String {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = abs.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Prints the page at `html` to `pdf` with Chrome: the page's own print
/// layout, its headings as the PDF's bookmarks, page numbers at the foot.
/// Chrome runs headless, with a throwaway profile, a time limit and no
/// network: images on the web are not fetched (`--embed` puts them inside).
pub(crate) fn pdf(html: &Path, pdf: &Path) -> Result<(), String> {
    let Some(chrome) = chrome() else {
        return Err(String::from(
            "a PDF needs Chrome or Chromium, and none was found (set MD2HTML_CHROME to its program)",
        ));
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let profile =
        std::env::temp_dir().join(format!("md2html-print-{}-{stamp}", std::process::id()));
    let name = pdf
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = pdf.with_file_name(format!(".{name}.md2html-tmp"));
    let mut child = Command::new(&chrome)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--disable-background-networking",
            "--disable-sync",
            "--disable-component-update",
            "--no-pdf-header-footer",
            "--generate-pdf-document-outline",
            "--virtual-time-budget=5000",
            // No network while printing: names resolve nowhere and every
            // connection goes to a closed port, so a page cannot reach out.
            "--host-resolver-rules=MAP * ~NOTFOUND",
            "--proxy-server=socks5://127.0.0.1:9",
            "--proxy-bypass-list=<-loopback>",
            "--disable-domain-reliability",
            "--no-service-autorun",
        ])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--print-to-pdf={}", tmp.display()))
        .arg(file_url(html))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting {}: {e}", chrome.display()))?;
    let started = Instant::now();
    let finished = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < PRINT_LIMIT => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = std::fs::remove_dir_all(&profile);
    let printed = std::fs::read(&tmp).unwrap_or_default();
    if finished.is_none() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "Chrome took longer than {}s to print",
            PRINT_LIMIT.as_secs()
        ));
    }
    if !printed.starts_with(b"%PDF-") {
        let _ = std::fs::remove_file(&tmp);
        return Err(String::from("Chrome printed no PDF"));
    }
    std::fs::rename(&tmp, pdf).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("writing {}: {e}", pdf.display())
    })
}

/// What an EPUB says about itself.
pub(crate) struct Book<'a> {
    /// The title, plain.
    pub(crate) title: &'a str,
    /// The language code.
    pub(crate) lang: &'a str,
    /// Written right to left.
    pub(crate) rtl: bool,
    /// The author, plain.
    pub(crate) author: Option<&'a str>,
    /// The article's own stylesheet (its diagrams).
    pub(crate) css: &'a str,
    /// Where the page's images are found.
    pub(crate) bases: &'a [&'a Path],
}

/// The page's chrome taken off for an e-reader, and what the reader's own
/// layout replaces.
const EPUB_STYLE: &str = "body{margin:0 4%;background:var(--bg);color:var(--text);font:1em/1.6 var(--sans)}\
article{padding:0}h2{margin-top:2em;padding-top:0;border-top:0;page-break-before:always}\
.anchor,.copy,.back,nav.pages{display:none}\
pre,table,figure,.callout,.diagram{page-break-inside:avoid}\
img,svg{max-width:100%;height:auto}figure{margin:1em 0}\
.term{text-decoration:underline dotted}.credit{margin-top:2em}";

/// The page as an EPUB 3: one chapter file with the article, the outline
/// as the table of contents, the stylesheet, the images. The second value
/// lists what could not be packed.
#[expect(
    clippy::too_many_lines,
    reason = "one dispatch over the kinds it handles, read top to bottom"
)]
pub(crate) fn epub(page: &str, book: &Book) -> (Vec<u8>, Vec<String>) {
    let mut problems = Vec::new();
    let article = between(page, "<article id=\"main\">", "</article></main>").unwrap_or_default();
    let outline = between(page, "<ol>", "</ol></nav>").unwrap_or_default();
    let body = strip_chrome(article);
    let (body, images, remote) = pack_images(&body, book.bases, &mut problems);
    let body = xhtml(&body)
        .replace("class=\"fnref\"", "class=\"fnref\" epub:type=\"noteref\"")
        .replace("<li id=\"fn-", "<li epub:type=\"footnote\" id=\"fn-");
    let body = namespaces(&body);
    let dir = if book.rtl { "rtl" } else { "ltr" };
    let lang = esc(book.lang);
    let title = esc(book.title);
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\" dir=\"{dir}\">\
<head><meta charset=\"utf-8\"/><title>{title}</title><link rel=\"stylesheet\" href=\"style.css\"/></head>\
<body><article id=\"main\">{body}</article></body></html>"
    );
    let toc = xhtml(&outline.replace("href=\"#", "href=\"content.xhtml#"));
    let nav = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\" dir=\"{dir}\">\
<head><meta charset=\"utf-8\"/><title>{title}</title><link rel=\"stylesheet\" href=\"style.css\"/></head>\
<body><nav epub:type=\"toc\" id=\"toc\"><h1>{title}</h1><ol><li><a href=\"content.xhtml\">{title}</a></li>{toc}</ol></nav></body></html>"
    );
    let style = format!(
        "{}{}{}{EPUB_STYLE}",
        crate::page::BASE_STYLE,
        crate::page::CONTENT,
        book.css
    );
    let mut properties = Vec::new();
    if content.contains("<svg") {
        properties.push("svg");
    }
    if content.contains("<math") {
        properties.push("mathml");
    }
    if remote {
        properties.push("remote-resources");
    }
    let props = if properties.is_empty() {
        String::new()
    } else {
        format!(" properties=\"{}\"", properties.join(" "))
    };
    let digest = sha256(format!("{}\n{}\n{content}", book.title, book.lang).as_bytes());
    let id = hex(&digest[..16]);
    let uuid = format!(
        "{}-{}-5{}-a{}-{}",
        &id[0..8],
        &id[8..12],
        &id[13..16],
        &id[17..20],
        &id[20..32]
    );
    let now = crate::text::now_secs();
    let (y, mo, d) = crate::text::civil(now);
    let secs = now % 86_400;
    let modified = format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    );
    let creator = book
        .author
        .map(|a| format!("<dc:creator>{}</dc:creator>", esc(a)))
        .unwrap_or_default();
    let mut manifest = format!(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\
<item id=\"content\" href=\"content.xhtml\" media-type=\"application/xhtml+xml\"{props}/>\
<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>"
    );
    for (n, (name, mime, _)) in images.iter().enumerate() {
        let _ = write!(
            manifest,
            "<item id=\"img{n}\" href=\"{name}\" media-type=\"{mime}\"/>"
        );
    }
    let progression = if book.rtl {
        " page-progression-direction=\"rtl\""
    } else {
        ""
    };
    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\" xml:lang=\"{lang}\" dir=\"{dir}\">\
<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:identifier id=\"uid\">urn:uuid:{uuid}</dc:identifier><dc:title>{title}</dc:title><dc:language>{lang}</dc:language>{creator}\
<meta property=\"dcterms:modified\">{modified}</meta></metadata>\
<manifest>{manifest}</manifest><spine{progression}><itemref idref=\"content\"/></spine></package>"
    );
    let container = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\
<rootfiles><rootfile full-path=\"OEBPS/package.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>";
    let mut zip = Zip::default();
    zip.add("mimetype", b"application/epub+zip");
    zip.add("META-INF/container.xml", container.as_bytes());
    zip.add("OEBPS/package.opf", opf.as_bytes());
    zip.add("OEBPS/nav.xhtml", nav.as_bytes());
    zip.add("OEBPS/content.xhtml", content.as_bytes());
    zip.add("OEBPS/style.css", style.as_bytes());
    for (name, _, bytes) in &images {
        zip.add(&format!("OEBPS/{name}"), bytes);
    }
    (zip.finish(), problems)
}

/// The text between two marks, the first of each.
fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let len = text[start..].find(close)?;
    Some(&text[start..start + len])
}

/// The article without what only works on the web page: heading anchors,
/// copy buttons, the previous/next links; other buttons give up their
/// contents.
fn strip_chrome(html: &str) -> String {
    let mut out = remove_all(html, "<a class=\"anchor\"", "</a>");
    out = remove_all(&out, "<button class=\"copy\"", "</button>");
    out = remove_all(&out, "<nav class=\"pages\"", "</nav>");
    // Any other button: keep what it holds.
    let mut kept = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(at) = rest.find("<button") {
        kept.push_str(&rest[..at]);
        rest = &rest[at..];
        match rest.find('>') {
            Some(end) => rest = &rest[end + 1..],
            None => break,
        }
    }
    kept.push_str(rest);
    kept.replace("</button>", "")
}

/// `html` with every stretch from `open` to the next `close` removed.
fn remove_all(html: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find(open) {
        out.push_str(&rest[..at]);
        let Some(len) = rest[at..].find(close) else {
            rest = "";
            break;
        };
        rest = &rest[at + len + close.len()..];
    }
    out.push_str(rest);
    out
}

/// An image packed into the book: its name there, its media type, its bytes.
type Packed = (String, &'static str, Vec<u8>);

/// The page's images moved into the book; `src` and links to them point
/// there. Images on the web stay where they are (the third value says
/// whether there were any).
fn pack_images(
    html: &str,
    bases: &[&Path],
    problems: &mut Vec<String>,
) -> (String, Vec<Packed>, bool) {
    let mut images: Vec<Packed> = Vec::new();
    let mut names: Vec<(String, String)> = Vec::new();
    let mut remote = false;
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find("<img") {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.find('>').map_or(rest.len(), |e| e + 1);
        let tag = &rest[..end];
        rest = &rest[end..];
        let Some(src) = between(tag, " src=\"", "\"") else {
            out.push_str(tag);
            continue;
        };
        if src.starts_with("http://") || src.starts_with("https://") {
            remote = true;
            out.push_str(tag);
            continue;
        }
        let packed = names.iter().find(|(s, _)| s == src).map(|(_, n)| n.clone());
        let name = packed.map_or_else(
            || match load_image(src, bases) {
                Ok((mime, ext, bytes)) => {
                    let name = format!("images/img-{}.{ext}", images.len() + 1);
                    images.push((name.clone(), mime, bytes));
                    names.push((src.to_owned(), name.clone()));
                    Some(name)
                }
                Err(why) => {
                    problems.push(format!("epub: an image is left out: {why}"));
                    None
                }
            },
            Some,
        );
        match name {
            Some(name) => out.push_str(&tag.replacen(
                &format!(" src=\"{src}\""),
                &format!(" src=\"{name}\""),
                1,
            )),
            None => out.push_str(tag),
        }
    }
    out.push_str(rest);
    // A link to an image now packed opens the packed copy.
    for (src, name) in &names {
        out = out.replace(&format!("href=\"{src}\""), &format!("href=\"{name}\""));
    }
    (out, images, remote)
}

/// An image's bytes from a `data:` URL or a file beside the page.
fn load_image(src: &str, bases: &[&Path]) -> Result<(&'static str, &'static str, Vec<u8>), String> {
    if let Some(data) = src.strip_prefix("data:") {
        let (head, payload) = data.split_once(',').ok_or("a data: URL without data")?;
        let mime = head.split(';').next().unwrap_or_default();
        let (mime, ext) =
            media(mime).ok_or_else(|| format!("{mime} is not an image an e-reader shows"))?;
        let bytes = if head.ends_with(";base64") {
            base64_decode(payload).ok_or("a data: URL that is not base64")?
        } else {
            unpercent(payload).into_bytes()
        };
        return Ok((mime, ext, bytes));
    }
    let path = unpercent(src.split(['?', '#']).next().unwrap_or_default());
    let ext = Path::new(&path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let (mime, ext) = media(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "",
    })
    .ok_or_else(|| format!("{path} is not an image an e-reader shows"))?;
    if Path::new(&path).is_absolute() || path.split(['/', '\\']).any(|part| part == "..") {
        return Err(format!("{path} is outside the page's folder"));
    }
    bases
        .iter()
        .find_map(|base| std::fs::read(base.join(&path)).ok())
        .map(|bytes| (mime, ext, bytes))
        .ok_or_else(|| format!("{path} was not found"))
}

/// An image media type and its file extension, for the types EPUB knows.
fn media(mime: &str) -> Option<(&'static str, &'static str)> {
    match mime {
        "image/png" => Some(("image/png", "png")),
        "image/jpeg" => Some(("image/jpeg", "jpg")),
        "image/gif" => Some(("image/gif", "gif")),
        "image/webp" => Some(("image/webp", "webp")),
        "image/svg+xml" => Some(("image/svg+xml", "svg")),
        _ => None,
    }
}

/// `%20` and its kind decoded.
fn unpercent(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Inline SVG and `MathML` with the namespaces XML needs (HTML implies them).
fn namespaces(xhtml: &str) -> String {
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

/// Elements HTML writes without an end tag.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// md2html's HTML as well-formed XHTML: void elements closed, every
/// attribute quoted and given a value, named character references other
/// than XML's five written as numbers, comments and scripts dropped.
pub(crate) fn xhtml(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / 16);
    let mut rest = html;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            text(rest, &mut out);
            break;
        };
        text(&rest[..at], &mut out);
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }
        if rest.starts_with("<script") {
            rest = rest.find("</script>").map_or("", |end| &rest[end + 9..]);
            continue;
        }
        let end = tag_end(rest);
        let tag = &rest[1..end - 1];
        rest = &rest[end..];
        if let Some(name) = tag.strip_prefix('/') {
            let _ = write!(out, "</{}>", name.trim());
            continue;
        }
        let closed = tag.ends_with('/');
        let tag = tag.strip_suffix('/').unwrap_or(tag);
        let name_end = tag.find(|c: char| c.is_whitespace()).unwrap_or(tag.len());
        let name = &tag[..name_end];
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':')
        {
            text(&format!("<{tag}>"), &mut out);
            continue;
        }
        out.push('<');
        out.push_str(name);
        for (key, value) in attributes(&tag[name_end..]) {
            let _ = write!(out, " {key}=\"");
            match value {
                Some(value) => attr_text(&value, &mut out),
                None => out.push_str(&key),
            }
            out.push('"');
        }
        let self_closing = closed || VOID.contains(&name.to_ascii_lowercase().as_str());
        out.push_str(if self_closing { "/>" } else { ">" });
    }
    out
}

/// Where a tag that starts `text` ends, past its `>`, quotes respected.
fn tag_end(text: &str) -> usize {
    let mut quote = None;
    for (i, ch) in text.char_indices().skip(1) {
        match (quote, ch) {
            (None, '"' | '\'') => quote = Some(ch),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return i + 1,
            _ => {}
        }
    }
    text.len()
}

/// A tag's attributes as (name, value); a bare attribute has no value.
fn attributes(text: &str) -> Vec<(String, Option<String>)> {
    let mut list: Vec<(String, Option<String>)> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '/') {
            i += 1;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '/' {
            i += 1;
        }
        if start == i {
            i += 1;
            continue;
        }
        let key: String = chars[start..i].iter().collect();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut value = None;
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let q = chars[i];
                i += 1;
                let s = i;
                while i < chars.len() && chars[i] != q {
                    i += 1;
                }
                value = Some(chars[s..i].iter().collect());
                i += 1;
            } else {
                let s = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                value = Some(chars[s..i].iter().collect());
            }
        }
        let valid = key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'));
        if valid && !list.iter().any(|(k, _)| *k == key) {
            list.push((key, value));
        }
    }
    list
}

/// Text with its character references made XML's.
fn text(raw: &str, out: &mut String) {
    let mut rest = raw;
    while let Some(at) = rest.find(['&', '<', '>']) {
        out.push_str(&rest[..at]);
        let ch = rest.as_bytes()[at];
        rest = &rest[at + 1..];
        match ch {
            b'<' => out.push_str("&lt;"),
            b'>' => out.push_str("&gt;"),
            _ => rest = reference(rest, out),
        }
    }
    out.push_str(rest);
}

/// An attribute value with its references made XML's and `<` escaped.
fn attr_text(raw: &str, out: &mut String) {
    let mut rest = raw;
    while let Some(at) = rest.find(['&', '<', '"']) {
        out.push_str(&rest[..at]);
        let ch = rest.as_bytes()[at];
        rest = &rest[at + 1..];
        match ch {
            b'<' => out.push_str("&lt;"),
            b'"' => out.push_str("&quot;"),
            _ => rest = reference(rest, out),
        }
    }
    out.push_str(rest);
}

/// The named references HTML pages use, as code points.
const NAMED: &[(&str, u32)] = &[
    ("nbsp", 0xA0),
    ("middot", 0xB7),
    ("mdash", 0x2014),
    ("ndash", 0x2013),
    ("hellip", 0x2026),
    ("lsaquo", 0x2039),
    ("rsaquo", 0x203A),
    ("laquo", 0xAB),
    ("raquo", 0xBB),
    ("ldquo", 0x201C),
    ("rdquo", 0x201D),
    ("lsquo", 0x2018),
    ("rsquo", 0x2019),
    ("copy", 0xA9),
    ("reg", 0xAE),
    ("trade", 0x2122),
    ("times", 0xD7),
    ("divide", 0xF7),
    ("larr", 0x2190),
    ("uarr", 0x2191),
    ("rarr", 0x2192),
    ("darr", 0x2193),
    ("harr", 0x2194),
    ("bull", 0x2022),
    ("deg", 0xB0),
    ("plusmn", 0xB1),
    ("sect", 0xA7),
    ("para", 0xB6),
    ("euro", 0x20AC),
    ("pound", 0xA3),
    ("shy", 0xAD),
    ("zwj", 0x200D),
    ("zwnj", 0x200C),
    ("thinsp", 0x2009),
    ("ensp", 0x2002),
    ("emsp", 0x2003),
];

/// After a `&`: the reference written for XML; the rest of the text.
fn reference<'a>(rest: &'a str, out: &mut String) -> &'a str {
    let end = rest
        .char_indices()
        .take(32)
        .find(|(_, c)| *c == ';' || !(c.is_ascii_alphanumeric() || *c == '#'))
        .filter(|(_, c)| *c == ';')
        .map(|(i, _)| i);
    let Some(end) = end else {
        out.push_str("&amp;");
        return rest;
    };
    let name = &rest[..end];
    let numeric = name
        .strip_prefix("#x")
        .or_else(|| name.strip_prefix("#X"))
        .map(|h| u32::from_str_radix(h, 16).ok())
        .or_else(|| name.strip_prefix('#').map(|d| d.parse::<u32>().ok()));
    match numeric {
        Some(Some(code)) if char::from_u32(code).is_some_and(xml_char) => {
            let _ = write!(out, "&#{code};");
        }
        Some(_) => out.push_str("&#xFFFD;"),
        None if ["amp", "lt", "gt", "quot", "apos"].contains(&name) => {
            let _ = write!(out, "&{name};");
        }
        None => {
            let Some((_, code)) = NAMED.iter().find(|(n, _)| *n == name) else {
                out.push_str("&amp;");
                return rest;
            };
            let _ = write!(out, "&#{code};");
        }
    }
    &rest[end + 1..]
}

/// A character XML 1.0 allows.
fn xml_char(ch: char) -> bool {
    matches!(u32::from(ch), 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x1_0000..=0x10_FFFF)
}

/// A zip archive of stored (uncompressed) entries, as EPUB allows; the
/// first entry, `mimetype`, must be stored.
#[derive(Default)]
struct Zip {
    /// The archive so far.
    bytes: Vec<u8>,
    /// The central directory so far.
    central: Vec<u8>,
    /// Entries written.
    count: u16,
}

impl Zip {
    /// Adds a file.
    fn add(&mut self, name: &str, data: &[u8]) {
        let crc = crc32(data);
        let size = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let offset = u32::try_from(self.bytes.len()).unwrap_or(u32::MAX);
        let name_len = u16::try_from(name.len()).unwrap_or(u16::MAX);
        // 1980-01-01 00:00: the same book from the same page, byte for byte.
        let (dos_time, dos_date) = (0u16, 0x21u16);
        let flags: u16 = if name.is_ascii() { 0 } else { 0x0800 };
        let local = &mut self.bytes;
        local.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
        for v in [10u16, flags, 0, dos_time, dos_date] {
            local.extend_from_slice(&v.to_le_bytes());
        }
        for v in [crc, size, size] {
            local.extend_from_slice(&v.to_le_bytes());
        }
        local.extend_from_slice(&name_len.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name.as_bytes());
        local.extend_from_slice(data);
        let c = &mut self.central;
        c.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
        for v in [20u16, 10, flags, 0, dos_time, dos_date] {
            c.extend_from_slice(&v.to_le_bytes());
        }
        for v in [crc, size, size] {
            c.extend_from_slice(&v.to_le_bytes());
        }
        for v in [name_len, 0, 0, 0, 0] {
            c.extend_from_slice(&v.to_le_bytes());
        }
        c.extend_from_slice(&0u32.to_le_bytes());
        c.extend_from_slice(&offset.to_le_bytes());
        c.extend_from_slice(name.as_bytes());
        self.count = self.count.saturating_add(1);
    }

    /// The archive, closed.
    fn finish(mut self) -> Vec<u8> {
        let start = u32::try_from(self.bytes.len()).unwrap_or(u32::MAX);
        let size = u32::try_from(self.central.len()).unwrap_or(u32::MAX);
        self.bytes.extend_from_slice(&self.central);
        self.bytes.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
        for v in [0u16, 0, self.count, self.count] {
            self.bytes.extend_from_slice(&v.to_le_bytes());
        }
        self.bytes.extend_from_slice(&size.to_le_bytes());
        self.bytes.extend_from_slice(&start.to_le_bytes());
        self.bytes.extend_from_slice(&0u16.to_le_bytes());
        self.bytes
    }
}

/// The CRC-32 a zip records for each file.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_of_a_known_text() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn html_becomes_well_formed_xhtml() {
        let out = xhtml(
            "<p>a&nbsp;b &middot; c & d<br><img src=\"x.png\" alt=a loading=lazy><input type=checkbox checked disabled></p><!-- gone -->",
        );
        assert_eq!(
            out,
            "<p>a&#160;b &#183; c &amp; d<br/><img src=\"x.png\" alt=\"a\" loading=\"lazy\"/><input type=\"checkbox\" checked=\"checked\" disabled=\"disabled\"/></p>"
        );
    }

    #[test]
    fn a_zip_starts_with_the_stored_mimetype() {
        let mut zip = Zip::default();
        zip.add("mimetype", b"application/epub+zip");
        let bytes = zip.finish();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert_eq!(&bytes[30..38], b"mimetype");
        assert_eq!(&bytes[38..58], b"application/epub+zip");
    }
}
