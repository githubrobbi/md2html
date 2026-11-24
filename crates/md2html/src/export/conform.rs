// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a web page may do and an EPUB may not, done the book's way.
//!
//! An outline list holds at least one entry, the style sheet leaves the
//! writing direction to the markup, and a link leads to a document in the
//! book or to the web.

use alloc::collections::BTreeSet;

use super::between;
use crate::scan::Scan;
use crate::text::{self, put};

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

/// The chapter with its code, keys, maths and diagrams marked `dir="ltr"`.
///
/// The reading system isolates such an element from the text around it:
/// what the web page's style sheet does with `direction`, said in the
/// markup. An element that names its own direction keeps it.
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

/// The chapter with every link that leads nowhere in a book taken off its text.
///
/// Such a link goes to a file beside the page (another page, a PDF, an
/// image), which the book does not hold, has a scheme a book cannot
/// follow, names a place the chapter does not have (a place in the
/// page around the article, or nowhere), or a host no reading system can
/// look up. Links to places in the chapter and to the web stay. The second
/// value names each target once, for the author; `quiet` lists targets that go
/// without a word (the book's own images, and those already reported as left
/// out).
pub(super) fn unlink(xhtml: &str, quiet: &[&str]) -> (String, Vec<String>) {
    let places = ids(xhtml);
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
        let dead = attribute(inside, "href")
            .map(|href| (href, leads(href, &places)))
            .filter(|(_, lead)| *lead != Lead::There);
        if let Some((href, lead)) = dead.filter(|(href, _)| !quiet.contains(href)) {
            let why = match lead {
                Lead::NoHost => "names no host a reading system can look up",
                Lead::There | Lead::Away => "is not in the book",
            };
            let problem = format!(
                "epub: a link is left out: {} {why}",
                text::decode_entities(href)
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

/// The chapter with every source that is not in the book left out.
///
/// An element that is nothing without its source (a `<source>`, a
/// `<track>`) goes, and an image gives way to its text alternative; any
/// other (a video, an audio, an image's other sizes) stays without it,
/// showing what it holds instead. `held` lists
/// the book's own files. The second value names each source left out once,
/// for the author.
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
                text::decode_entities(value)
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
        // An image is its source: one without (the author's `<img>` with
        // none, or none in the book) gives way to its text alternative, as
        // XHTML refuses it.
        let sourceless = name.eq_ignore_ascii_case("img") && attribute(&kept, "src").is_none();
        if sourceless {
            out.push_str(attribute(&kept, "alt").unwrap_or_default());
            let problem = String::from("epub: an <img> without a source is left out");
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
        if !emptied && !sourceless {
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

/// Where a link of the chapter leads, in a book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lead {
    /// Where it should: a place in the chapter, or the web or mail.
    There,
    /// Out of the book: a file beside the page, a place the chapter does
    /// not have, a scheme a book cannot follow.
    Away,
    /// To a host no reading system can look up (a part of its name over
    /// 63 characters, or the name over 253), which EPUB's validator does
    /// not take for a URL.
    NoHost,
}

/// Where a link leads in a book: to a place in the chapter (one of
/// `places`, its ids), or by a scheme a reading system can hand on (the
/// web, mail) to a host it can look up; or away.
fn leads(href: &str, places: &BTreeSet<&str>) -> Lead {
    if href.is_empty() {
        return Lead::There;
    }
    if let Some(fragment) = href.strip_prefix('#') {
        // As a reading system finds the place: by the id written, or by
        // what its escapes stand for.
        let id = text::decode_entities(fragment);
        let unescaped = String::from_utf8(text::percent_decode(&id)).ok();
        let found = id.is_empty()
            || places.contains(id.as_str())
            || places.contains(text::url_attr(&id).as_str())
            || unescaped.is_some_and(|place| places.contains(place.as_str()));
        return if found { Lead::There } else { Lead::Away };
    }
    let handed_on = scheme(href).is_some_and(|scheme| {
        !DEAD_SCHEMES
            .iter()
            .any(|dead| scheme.eq_ignore_ascii_case(dead))
    });
    if !handed_on {
        Lead::Away
    } else if host_fits(&text::decode_entities(href)) {
        Lead::There
    } else {
        Lead::NoHost
    }
}

/// Whether the host a URL names (`scheme://host/...`), if any, fits DNS.
///
/// Each part of the name is 1 to 63 characters, the name at most 253. A
/// name in another script is measured in its ASCII form, which this does
/// not make, so it passes; so does an address in brackets.
fn host_fits(url: &str) -> bool {
    let Some(rest) = url
        .split_once(':')
        .and_then(|(_, rest)| rest.strip_prefix("//"))
    else {
        return true;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let place = authority
        .rsplit_once('@')
        .map_or(authority, |(_, place)| place);
    if place.starts_with('[') {
        return true;
    }
    let host = place.split_once(':').map_or(place, |(host, _)| host);
    let name = host.strip_suffix('.').unwrap_or(host);
    !name.is_ascii()
        || (name.len() <= 253 && name.split('.').all(|part| (1..=63).contains(&part.len())))
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
pub(super) fn each_tag<Write>(xhtml: &str, mut write: Write) -> String
where
    Write: FnMut(&str, &mut String),
{
    let mut out = String::with_capacity(xhtml.len() + xhtml.len() / 32);
    let mut scan = Scan::new(xhtml);
    while let Some(before) = scan.take_until("<") {
        out.push_str(before);
        write(next_tag(&mut scan), &mut out);
    }
    out.push_str(scan.take_rest());
    out
}

/// Reads the tag at `<`, past its `>`, and gives what stands between.
pub(super) fn next_tag<'text>(scan: &mut Scan<'text>) -> &'text str {
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
    inside
}

/// The ids of the chapter's elements: the places a link may lead to.
fn ids(xhtml: &str) -> BTreeSet<&str> {
    let mut places = BTreeSet::new();
    let mut scan = Scan::new(xhtml);
    while scan.take_until("<").is_some() {
        places.extend(attribute(next_tag(&mut scan), "id"));
    }
    places
}

/// A tag's name, from what stands between its `<` and `>`.
pub(super) fn tag_name(inside: &str) -> &str {
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
    /// A link to a host is kept only when the host's name fits DNS.
    ///
    /// # Panics
    ///
    /// When a link whose host has a part of 64 characters, a name over
    /// 253, or no name stays a link, or is not named once; when a link to
    /// a host that fits (with a user, a port, a trailing dot, in brackets,
    /// in another script) or to mail is lost.
    #[test]
    fn a_link_to_a_host_no_reader_can_look_up_keeps_its_text() {
        let part = "a".repeat(64);
        let long = format!("{}.b", "abc.".repeat(64));
        let html = format!(
            "<a href=\"http://www.{part}.com/x\">part</a> <a href=\"https://{long}/\">long</a> \
<a href=\"http:///x\">none</a> <a href=\"http://www.{part}.com/x\">again</a> \
<a href=\"https://u@example.com:8080/a?b#c\">ok</a> <a href=\"https://example.com./\">dot</a> \
<a href=\"http://[::1]/\">six</a> <a href=\"https://bücher.example/\">idn</a> <a href=\"mailto:a@example.com\">mail</a>"
        );
        let (out, problems) = unlink(&html, &[]);
        assert_eq!(
            out,
            "part long none again <a href=\"https://u@example.com:8080/a?b#c\">ok</a> \
<a href=\"https://example.com./\">dot</a> <a href=\"http://[::1]/\">six</a> \
<a href=\"https://bücher.example/\">idn</a> <a href=\"mailto:a@example.com\">mail</a>"
        );
        assert_eq!(problems, [
            format!(
                "epub: a link is left out: http://www.{part}.com/x names no host a reading system can look up"
            ),
            format!(
                "epub: a link is left out: https://{long}/ names no host a reading system can look up"
            ),
            String::from(
                "epub: a link is left out: http:///x names no host a reading system can look up"
            ),
        ]);
    }

    use super::{left_to_right, outline, style, unlink, unsourced};

    /// An outline writes a chapter's list of sections only when it has some.
    ///
    /// # Panics
    ///
    /// Panics when an empty `<ol class="sub"></ol>` stays in the outline, when
    /// a list with entries is lost, or when an `<ol` that never closes is
    /// changed.
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

    /// The book's style sheet holds no declaration EPUB forbids.
    ///
    /// # Panics
    ///
    /// Panics when a `direction` or `unicode-bidi` declaration stays (in any
    /// case, inside an at-rule too), when a rule they emptied keeps its
    /// selector or the statement before it, or when any other declaration
    /// changes.
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

    /// Code, keys, maths and diagrams say in the markup that they run left to
    /// right.
    ///
    /// # Panics
    ///
    /// Panics when a `pre`, `code`, `kbd`, `.math` or `.diagram` element lacks
    /// `dir="ltr"`, when `.diagram-js` or a paragraph gets it, or when an
    /// element that names its own direction loses it.
    #[test]
    fn code_keys_maths_and_diagrams_run_left_to_right() {
        assert_eq!(
            left_to_right(
                "<pre class=\"x\"><code>a</code></pre><kbd>K</kbd><div class=\"math\"></div><figure id=\"f\" class=\"diagram wide\"></figure><div class=\"diagram-js\" title=\"a > b\"></div><code dir=\"rtl\">c</code><p>p</p><br/>"
            ),
            "<pre dir=\"ltr\" class=\"x\"><code dir=\"ltr\">a</code></pre><kbd dir=\"ltr\">K</kbd><div dir=\"ltr\" class=\"math\"></div><figure dir=\"ltr\" id=\"f\" class=\"diagram wide\"></figure><div class=\"diagram-js\" title=\"a > b\"></div><code dir=\"rtl\">c</code><p>p</p><br/>"
        );
    }

    /// A link to what the book does not hold keeps its text and loses the link.
    ///
    /// # Panics
    ///
    /// Panics when a link to another page, a PDF, a `file:` address or a
    /// place the chapter does not have stays, when a link to a place in
    /// the chapter (by its id, or by its escapes), to the web or to mail is
    /// lost, when a named anchor changes, or when the targets are not named
    /// once each.
    #[test]
    fn a_link_out_of_the_book_keeps_only_its_text() {
        let (out, problems) = unlink(
            "<a href=\"SECURITY.html\">policy</a>, <a href=\"docs/a.html#x\">a</a>, <a href=\"SECURITY.html\"><b>again</b></a>, \
<a href=\"#spot\">own</a>, <a href=\"https://example.com/?a=1&amp;b=2\">web</a>, <a href=\"mailto:a@example.com\">mail</a>, \
<a href=\"file:///etc/hosts\">file</a>, <a href=\"manual.pdf\">pdf</a>, <a id=\"spot\"></a>, <a href=\"x&amp;y.md\">amp</a>, \
<a href=\"#top\">top</a>, <a href=\"#caf%C3%A9\">café</a><h2 id=\"café\">x</h2><a href=\"#\">start</a>\
<a href=\"#a%20b\">space</a><a id=\"a b\"></a>",
            &[],
        );
        assert_eq!(
            out,
            "policy, a, <b>again</b>, <a href=\"#spot\">own</a>, <a href=\"https://example.com/?a=1&amp;b=2\">web</a>, \
<a href=\"mailto:a@example.com\">mail</a>, file, pdf, <a id=\"spot\"></a>, amp, \
top, <a href=\"#caf%C3%A9\">café</a><h2 id=\"café\">x</h2><a href=\"#\">start</a>\
<a href=\"#a%20b\">space</a><a id=\"a b\"></a>"
        );
        assert_eq!(problems, [
            "epub: a link is left out: SECURITY.html is not in the book",
            "epub: a link is left out: docs/a.html#x is not in the book",
            "epub: a link is left out: file:///etc/hosts is not in the book",
            "epub: a link is left out: manual.pdf is not in the book",
            "epub: a link is left out: x&y.md is not in the book",
            "epub: a link is left out: #top is not in the book",
        ]);
    }

    /// A figure shows its image in the book and no longer links to it.
    ///
    /// # Panics
    ///
    /// Panics when the link around the book's own image stays, or when it is
    /// reported.
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

    /// A video's, audio's or picture's source that is not in the book is left
    /// out.
    ///
    /// # Panics
    ///
    /// Panics when a `src`, `srcset` or `poster` naming a file outside the book
    /// stays, when a `<source>`, a `<track>` or an `<img>` without its source
    /// stays, when a source in the book is lost, or when the problems are not
    /// named once each.
    #[test]
    fn a_source_out_of_the_book_is_left_out() {
        let (out, problems) = unsourced(
            "<video src=\"clip.mp4\" poster=\"images/img-1.png\" controls=\"controls\">Watch <a href=\"#x\">it</a></video>\
<audio src=\"https://example.com/a.mp3\"><track src=\"a.vtt\"/></audio>\
<picture><source srcset=\"a.svg\" type=\"image/svg+xml\"/><img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x, b.png 2x\" sizes=\"50vw\" alt=\"a\"/></picture>\
<img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x\"/><img alt=\"none\"/><img/>",
            &["images/img-1.png"],
        );
        assert_eq!(
            out,
            "<video poster=\"images/img-1.png\" controls=\"controls\">Watch <a href=\"#x\">it</a></video>\
<audio></audio><picture><img src=\"images/img-1.png\" alt=\"a\"/></picture>\
<img src=\"images/img-1.png\" srcset=\"images/img-1.png 1x\"/>none"
        );
        assert_eq!(problems, [
            "epub: <video src> is left out: clip.mp4 is not in the book",
            "epub: <audio src> is left out: https://example.com/a.mp3 is not in the book",
            "epub: <track src> is left out: a.vtt is not in the book",
            "epub: <source srcset> is left out: a.svg is not in the book",
            "epub: <img srcset> is left out: images/img-1.png 1x, b.png 2x is not in the book",
            "epub: an <img> without a source is left out",
        ]);
    }
}
