// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a web page may do and an EPUB may not, done the book's way: an
//! outline list holds at least one entry, the style sheet leaves the
//! writing direction to the markup, and a link leads to a document in the
//! book or to the web.

use super::between;
use crate::scan::Scan;
use crate::text::put;

/// Properties an EPUB style sheet may not hold: the reading system sets
/// the writing direction from the markup's `dir` alone.
const FORBIDDEN: [&str; 2] = ["direction", "unicode-bidi"];

/// Elements that run left to right in any book: code and keys.
const LTR_ELEMENTS: [&str; 3] = ["pre", "code", "kbd"];

/// Classes whose elements run left to right in any book: maths and
/// diagrams.
const LTR_CLASSES: [&str; 2] = ["math", "diagram"];

/// Attributes that name what an element shows or plays.
const SOURCES: [&str; 3] = ["src", "srcset", "poster"];

/// Elements that are nothing without their source.
const SOURCE_ONLY: [&str; 2] = ["source", "track"];

/// Schemes a link may not use in a book: files on the machine that made
/// it, data and script.
const DEAD_SCHEMES: [&str; 4] = ["file", "data", "javascript", "vbscript"];

/// The outline without empty lists: a chapter without sections has no
/// list of them, since an `ol` in EPUB's XHTML holds at least one `li`.
pub(super) fn outline(toc: &str) -> String {
    let mut out = String::with_capacity(toc.len());
    let mut scan = Scan::new(toc);
    while let Some(before) = scan.take_until("<ol") {
        out.push_str(before);
        let start = scan.mark();
        if scan.take_until(">").is_none() {
            break;
        }
        scan.eat('>');
        let open = scan.since(start);
        if !scan.eat_str("</ol>") {
            out.push_str(open);
        }
    }
    out.push_str(scan.take_rest());
    out
}

/// `css` without the declarations EPUB forbids; a rule left with none
/// goes too. The style sheets are md2html's own: no comment or string in
/// them holds a brace.
pub(super) fn style(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut scan = Scan::new(css);
    loop {
        let before = scan.take_while(|ch| ch != '{' && ch != '}');
        match scan.bump() {
            None => {
                out.push_str(before);
                return out;
            }
            Some('{') => {
                let body_start = scan.mark();
                let body = scan.take_while(|ch| ch != '{' && ch != '}');
                if scan.eat('}') {
                    rule(before, body, &mut out);
                } else {
                    // An at-rule: the rules inside it come next.
                    put!(out, "{before}{{");
                    scan.rewind(body_start);
                }
            }
            Some(close) => put!(out, "{before}{close}"),
        }
    }
}

/// Writes one rule, `selector{body}`, without the declarations EPUB
/// forbids. A rule they were all of is left out, and only the selector
/// goes with it, not a statement before it.
fn rule(selector: &str, body: &str, out: &mut String) {
    let kept: Vec<&str> = body.split(';').filter(|decl| !forbidden(decl)).collect();
    let emptied = !body.trim().is_empty() && kept.iter().all(|decl| decl.trim().is_empty());
    if emptied {
        if let Some((statements, _)) = selector.rsplit_once(';') {
            put!(out, "{statements};");
        }
    } else {
        put!(out, "{selector}{{{}}}", kept.join(";"));
    }
}

/// Whether a declaration sets a property EPUB forbids.
fn forbidden(decl: &str) -> bool {
    decl.split_once(':').is_some_and(|(property, _)| {
        FORBIDDEN
            .iter()
            .any(|name| property.trim().eq_ignore_ascii_case(name))
    })
}

/// The chapter with its code, keys, maths and diagrams marked `dir="ltr"`,
/// which the reading system isolates from the text around them: what the
/// web page's style sheet does with `direction`, said in the markup. An
/// element that names its own direction keeps it.
pub(super) fn left_to_right(xhtml: &str) -> String {
    each_tag(xhtml, |inside, out| {
        let name = tag_name(inside);
        let ltr = !inside.starts_with('/')
            && attribute(inside, "dir").is_none()
            && (LTR_ELEMENTS.contains(&name)
                || attribute(inside, "class").is_some_and(|class| {
                    class
                        .split_whitespace()
                        .any(|token| LTR_CLASSES.contains(&token))
                }));
        match inside.strip_prefix(name) {
            Some(rest) if ltr => put!(out, "<{name} dir=\"ltr\"{rest}>"),
            Some(_) | None => put!(out, "<{inside}>"),
        }
    })
}

/// The chapter with every link that leads nowhere in a book taken off
/// its text: one to a file beside the page (another page, a PDF, an
/// image), which the book does not hold, or with a scheme a book cannot
/// follow. Links within the chapter and to the web stay. The second
/// value names each target once, for the author; `quiet` lists targets
/// that go without a word (the book's own images, and those already
/// reported as left out).
pub(super) fn unlink(xhtml: &str, quiet: &[&str]) -> (String, Vec<String>) {
    let mut open: Vec<bool> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    let out = each_tag(xhtml, |inside, out| {
        if !tag_name(inside).eq_ignore_ascii_case("a") {
            put!(out, "<{inside}>");
            return;
        }
        if inside.starts_with('/') {
            if open.pop().unwrap_or(true) {
                put!(out, "<{inside}>");
            }
            return;
        }
        let dead = attribute(inside, "href").filter(|href| !reaches(href));
        if let Some(href) = dead.filter(|href| !quiet.contains(href)) {
            let problem = format!(
                "epub: a link is left out: {} is not in the book",
                crate::inline::decode_entities(href)
            );
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
        if !inside.ends_with('/') {
            open.push(dead.is_none());
        }
        if dead.is_none() {
            put!(out, "<{inside}>");
        }
    });
    (out, problems)
}

/// The chapter with every source that is not in the book left out: an
/// element that is nothing without it (a `<source>`, a `<track>`) goes;
/// any other (a video, an audio, an image's other sizes) stays without it,
/// showing what it holds instead. `held` lists the book's own files. The
/// second value names each source left out once, for the author.
pub(super) fn unsourced(xhtml: &str, held: &[&str]) -> (String, Vec<String>) {
    let mut problems: Vec<String> = Vec::new();
    let out = each_tag(xhtml, |inside, out| {
        let name = tag_name(inside);
        let mut kept = inside.to_owned();
        for attr in SOURCES {
            let Some(value) = attribute(inside, attr).filter(|value| !in_book(attr, value, held))
            else {
                continue;
            };
            let problem = format!(
                "epub: <{name} {attr}> is left out: {} is not in the book",
                crate::inline::decode_entities(value)
            );
            if !problems.contains(&problem) {
                problems.push(problem);
            }
            kept = kept.replacen(&format!(" {attr}=\"{value}\""), "", 1);
            if attr == "srcset" {
                // The sizes say which of the set to take: no set, no sizes.
                if let Some(sizes) = attribute(inside, "sizes") {
                    kept = kept.replacen(&format!(" sizes=\"{sizes}\""), "", 1);
                }
            }
        }
        let emptied = kept.len() < inside.len() && SOURCE_ONLY.contains(&name);
        if !emptied {
            put!(out, "<{kept}>");
        }
    });
    (out, problems)
}

/// Whether every file a source attribute names is one of the book's: a
/// `srcset` names one per entry, before its width or density.
fn in_book(attr: &str, value: &str, held: &[&str]) -> bool {
    if attr == "srcset" {
        value.split(',').all(|entry| {
            entry
                .split_whitespace()
                .next()
                .is_some_and(|url| held.contains(&url))
        })
    } else {
        held.contains(&value)
    }
}

/// Whether a link works in a book: one within the chapter, or one with a
/// scheme a reading system can hand on (the web, mail).
fn reaches(href: &str) -> bool {
    if href.is_empty() || href.starts_with('#') {
        return true;
    }
    scheme(href).is_some_and(|scheme| {
        !DEAD_SCHEMES
            .iter()
            .any(|dead| scheme.eq_ignore_ascii_case(dead))
    })
}

/// A URL's scheme: a letter, then letters, digits, `+`, `-` or `.`,
/// before a `:`.
fn scheme(url: &str) -> Option<&str> {
    let (scheme, _) = url.split_once(':')?;
    let mut chars = scheme.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'));
    valid.then_some(scheme)
}

/// Copies well-formed XHTML, giving each tag (what stands between its `<`
/// and `>`; a `>` inside a quoted value does not end it) to `write`,
/// which writes it as it should be.
fn each_tag<Write>(xhtml: &str, mut write: Write) -> String
where
    Write: FnMut(&str, &mut String),
{
    let mut out = String::with_capacity(xhtml.len() + xhtml.len() / 32);
    let mut scan = Scan::new(xhtml);
    while let Some(before) = scan.take_until("<") {
        out.push_str(before);
        scan.eat('<');
        let start = scan.mark();
        let mut quoted = false;
        while let Some(ch) = scan.peek() {
            if ch == '>' && !quoted {
                break;
            }
            quoted ^= ch == '"';
            scan.bump();
        }
        let inside = scan.since(start);
        scan.eat('>');
        write(inside, &mut out);
    }
    out.push_str(scan.take_rest());
    out
}

/// A tag's name, from what stands between its `<` and `>`.
fn tag_name(inside: &str) -> &str {
    inside
        .trim_start_matches('/')
        .split(|ch: char| ch.is_whitespace() || ch == '/')
        .next()
        .unwrap_or_default()
}

/// The value of a tag's attribute, as XHTML writes it: double-quoted,
/// with a quote inside the value written `&quot;`.
fn attribute<'tag>(inside: &'tag str, name: &str) -> Option<&'tag str> {
    between(inside, &format!(" {name}=\""), "\"")
}

#[cfg(test)]
mod tests {
    use super::{left_to_right, outline, style, unlink, unsourced};

    #[test]
    fn a_chapter_without_sections_lists_none() {
        assert_eq!(
            outline(
                "<li><a href=\"#a\">A</a><ol class=\"sub\"></ol></li><li><a href=\"#b\">B</a><ol class=\"sub\"><li>b</li></ol></li>"
            ),
            "<li><a href=\"#a\">A</a></li><li><a href=\"#b\">B</a><ol class=\"sub\"><li>b</li></ol></li>"
        );
        assert_eq!(outline("a<ol class=\"sub\""), "a<ol class=\"sub\"");
    }

    #[test]
    fn the_style_sheet_leaves_direction_to_the_markup() {
        let css = "@charset \"utf-8\";[dir=rtl] pre,[dir=rtl] code{direction:ltr;unicode-bidi:isolate}\
[dir=rtl] pre{text-align:left}.a{flex-direction:row;Direction: rtl;color:red}\
@media print{.b{unicode-bidi:embed}.c{x:y}}.d{}";
        assert_eq!(
            style(css),
            "@charset \"utf-8\";[dir=rtl] pre{text-align:left}.a{flex-direction:row;color:red}\
@media print{.c{x:y}}.d{}"
        );
    }

    #[test]
    fn code_keys_maths_and_diagrams_run_left_to_right() {
        assert_eq!(
            left_to_right(
                "<pre class=\"x\"><code>a</code></pre><kbd>K</kbd><div class=\"math\"></div><figure id=\"f\" class=\"diagram wide\"></figure><div class=\"diagram-js\" title=\"a > b\"></div><code dir=\"rtl\">c</code><p>p</p><br/>"
            ),
            "<pre dir=\"ltr\" class=\"x\"><code dir=\"ltr\">a</code></pre><kbd dir=\"ltr\">K</kbd><div dir=\"ltr\" class=\"math\"></div><figure dir=\"ltr\" id=\"f\" class=\"diagram wide\"></figure><div class=\"diagram-js\" title=\"a > b\"></div><code dir=\"rtl\">c</code><p>p</p><br/>"
        );
    }

    #[test]
    fn a_link_out_of_the_book_keeps_only_its_text() {
        let (out, problems) = unlink(
            "<a href=\"SECURITY.html\">policy</a>, <a href=\"docs/a.html#x\">a</a>, <a href=\"SECURITY.html\"><b>again</b></a>, \
<a href=\"#own\">own</a>, <a href=\"https://example.com/?a=1&amp;b=2\">web</a>, <a href=\"mailto:a@example.com\">mail</a>, \
<a href=\"file:///etc/hosts\">file</a>, <a href=\"manual.pdf\">pdf</a>, <a id=\"spot\"></a>, <a href=\"x&amp;y.md\">amp</a>",
            &[],
        );
        assert_eq!(
            out,
            "policy, a, <b>again</b>, <a href=\"#own\">own</a>, <a href=\"https://example.com/?a=1&amp;b=2\">web</a>, \
<a href=\"mailto:a@example.com\">mail</a>, file, pdf, <a id=\"spot\"></a>, amp"
        );
        assert_eq!(problems, [
            "epub: a link is left out: SECURITY.html is not in the book",
            "epub: a link is left out: docs/a.html#x is not in the book",
            "epub: a link is left out: file:///etc/hosts is not in the book",
            "epub: a link is left out: manual.pdf is not in the book",
            "epub: a link is left out: x&y.md is not in the book",
        ]);
    }

    #[test]
    fn an_image_in_the_book_is_shown_not_linked_to() {
        let (out, problems) = unlink(
            "<figure class=\"fig\"><a href=\"images/img-1.jpg\" target=\"_blank\"><img src=\"images/img-1.jpg\" alt=\"a\"/></a></figure>",
            &["images/img-1.jpg"],
        );
        assert_eq!(
            out,
            "<figure class=\"fig\"><img src=\"images/img-1.jpg\" alt=\"a\"/></figure>"
        );
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn a_source_out_of_the_book_is_left_out() {
        let (out, problems) = unsourced(
            "<video src=\"clip.mp4\" poster=\"images/img-1.png\" controls=\"controls\">Watch <a href=\"#x\">it</a></video>\
<audio src=\"https://example.com/a.mp3\"><track src=\"a.vtt\"/></audio>\
<picture><source srcset=\"a.svg\" type=\"image/svg+xml\"/><img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x, b.png 2x\" sizes=\"50vw\" alt=\"a\"/></picture>\
<img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x\"/>",
            &["images/img-1.png"],
        );
        assert_eq!(
            out,
            "<video poster=\"images/img-1.png\" controls=\"controls\">Watch <a href=\"#x\">it</a></video>\
<audio></audio><picture><img src=\"images/img-1.png\" alt=\"a\"/></picture>\
<img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x\"/>"
        );
        assert_eq!(problems, [
            "epub: <video src> is left out: clip.mp4 is not in the book",
            "epub: <audio src> is left out: https://example.com/a.mp3 is not in the book",
            "epub: <track src> is left out: a.vtt is not in the book",
            "epub: <source srcset> is left out: a.svg is not in the book",
            "epub: <img srcset> is left out: images/img-1.png 1x, b.png 2x is not in the book",
        ]);
    }
}
