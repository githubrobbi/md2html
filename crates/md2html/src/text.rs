// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Small text helpers every part of the page uses: escaping and its
//! reverse (character references, backslash escapes), anchors, HTML back
//! to words, URLs, the date, and the encodings of bytes.

mod base64;
mod date;
mod entity;
mod html;
mod sha256;

pub(crate) use self::base64::{base64, base64_decode};
pub(crate) use self::date::{Date, civil, iso_date};
pub(crate) use self::entity::{decode_entities, entity, unescape};
pub(crate) use self::html::Html;
pub(crate) use self::sha256::sha256;

/// Appends formatted text to a `String`: `write!` without a result to
/// discard at every call. The first argument is the `String` (or a
/// `&mut String`), the rest is what `format!` takes.
macro_rules! put {
    ($out:expr, $($arg:tt)*) => {{
        use $crate::text::Put as _;
        $out.put_fmt(format_args!($($arg)*))
    }};
}
pub(crate) use put;

/// What [`put!`] calls: formatted text, appended.
pub(crate) trait Put {
    /// Appends what `args` formats.
    fn put_fmt(&mut self, args: core::fmt::Arguments<'_>);
}

impl Put for String {
    fn put_fmt(&mut self, args: core::fmt::Arguments<'_>) {
        // A `String` takes every write. An error can only come from a
        // `Display` that reports one itself; what it wrote before stays.
        match core::fmt::Write::write_fmt(self, args) {
            Ok(()) | Err(core::fmt::Error) => {}
        }
    }
}

/// `bytes` read from the front in whole runs of `N`; what is too short
/// for one more run stays in `bytes`.
fn runs<'bytes, const N: usize>(bytes: &mut &'bytes [u8]) -> impl Iterator<Item = &'bytes [u8; N]> {
    core::iter::from_fn(move || {
        let (run, rest) = bytes.split_first_chunk::<N>()?;
        *bytes = rest;
        Some(run)
    })
}

/// Escapes text for HTML, attribute values included.
pub(crate) fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    esc_into(&mut out, text);
    out
}

/// Writes text to `out` escaped for HTML, attribute values included:
/// [`esc`] for text that goes straight into the page, without a string
/// of its own on the way.
pub(crate) fn esc_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
}

/// A string as a JSON literal, `<` escaped so it cannot close a script.
pub(crate) fn json_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    json_str_into(&mut out, text);
    out
}

/// Writes a string to `out` as a JSON literal, `<` escaped so it cannot
/// close a script: [`json_str`] without a string of its own.
pub(crate) fn json_str_into(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' | '\t' => out.push(' '),
            '<' => out.push_str("\\u003c"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other if u32::from(other) < 0x20 => {}
            other => out.push(other),
        }
    }
    out.push('"');
}

/// An anchor from a heading or a term: letters and digits lowercased, any
/// other run a single dash. Letters beyond ASCII stay, so a German
/// heading keeps its "ü".
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut dash = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Rendered HTML back to words, for the search index and descriptions.
///
/// A tag that ends a cell, a row or an item ends a word too: without the
/// space, the last word of one table cell ran into the first of the next.
pub(crate) fn plain(html: &str) -> String {
    const BREAKS: [&str; 12] = [
        "/td",
        "/th",
        "/tr",
        "/li",
        "/p",
        "/dd",
        "/dt",
        "br",
        "/h2",
        "/h3",
        "/h4",
        "/figcaption",
    ];
    let mut out = String::with_capacity(html.len());
    // Where the inside of the tag being read begins, just after its `<`:
    // only its name is read, never copied.
    let mut tag: Option<usize> = None;
    for (at, ch) in html.char_indices() {
        match (ch, tag) {
            ('<', None) => tag = Some(at + '<'.len_utf8()),
            ('>', Some(inside)) => {
                let breaks = html
                    .get(inside..at)
                    .and_then(|tag_text| tag_text.split_whitespace().next())
                    .is_some_and(|head| BREAKS.contains(&head));
                if breaks {
                    out.push(' ');
                }
                tag = None;
            }
            (_, Some(_)) => {}
            (other, None) => out.push(other),
        }
    }
    let decoded = decode_entities(&out);
    let mut words = String::with_capacity(decoded.len());
    for word in decoded.split_whitespace() {
        if !words.is_empty() {
            words.push(' ');
        }
        words.push_str(word);
    }
    words
}

/// A URL made safe to write in an attribute: characters a URL may not
/// hold are percent-encoded (an existing `%XX` stays), then HTML-escaped.
pub(crate) fn url_attr(url: &str) -> String {
    const KEEP: &str = "-._~:/?#@!$&'()*+,;=%";
    let mut out = String::with_capacity(url.len());
    for ch in url.chars() {
        if ch.is_ascii_alphanumeric() || KEEP.contains(ch) {
            out.push(ch);
        } else {
            let mut buf = [0_u8; 4];
            for byte in ch.encode_utf8(&mut buf).bytes() {
                put!(out, "%{byte:02X}");
            }
        }
    }
    esc(&out)
}

/// What a URL is for: a link to follow, or an image's pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UrlFor {
    /// A link, a stylesheet, anything but pixels.
    Link,
    /// An image's pixels: a `data:` URL of an image may carry them.
    Image,
}

/// Whether a link or image destination may be written into the page.
/// Script URLs never; `data:` only for an image's pixels.
pub(crate) fn safe_url(url: &str, url_for: UrlFor) -> bool {
    let cleaned = url
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_control())
        .collect::<String>()
        .to_ascii_lowercase();
    let Some((scheme, _)) = cleaned.split_once(':') else {
        return true;
    };
    if scheme.contains(['/', '?', '#']) {
        return true;
    }
    match scheme {
        "javascript" | "vbscript" | "livescript" | "mocha" => false,
        "data" => {
            url_for == UrlFor::Image
                && [
                    "data:image/png",
                    "data:image/jpeg",
                    "data:image/jpg",
                    "data:image/gif",
                    "data:image/webp",
                    "data:image/avif",
                    "data:image/svg+xml",
                ]
                .iter()
                .any(|ok| cleaned.starts_with(ok))
        }
        _ => true,
    }
}

/// A URL that [`safe_url`] let through for what it is for.
///
/// Only [`SafeUrl::check`] and [`SafeUrl::anchor`] make one, so the page
/// that writes it does not ask again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SafeUrl(String);

impl SafeUrl {
    /// `url`, when a page may carry it for `url_for`.
    #[must_use]
    pub(crate) fn check(url: &str, url_for: UrlFor) -> Option<Self> {
        safe_url(url, url_for).then(|| Self(url.to_owned()))
    }

    /// A link to the place `id` on the same page, which is always safe.
    #[must_use]
    pub(crate) fn anchor(id: &str) -> Self {
        Self(format!("#{id}"))
    }

    /// The URL as an attribute's value ([`url_attr`]).
    #[must_use]
    pub(crate) fn attr(&self) -> Html {
        Html::markup(url_attr(&self.0))
    }
}

/// Whether a destination points into the page or the file system next to
/// it rather than somewhere with a scheme.
pub(crate) fn is_relative(url: &str) -> bool {
    !url.starts_with('#')
        && !url.starts_with("//")
        && url
            .split_once(':')
            .is_none_or(|(scheme, _)| scheme.contains(['/', '?', '#']))
}

/// The moment a page is made, in seconds since 1970: `SOURCE_DATE_EPOCH`
/// when it is set, so a build can be reproduced byte for byte, else now.
///
/// # Errors
///
/// [`Unpinned`] when `SOURCE_DATE_EPOCH` is set to something other than a
/// whole number of seconds: the time of the run is used, and the person
/// who set the variable for a reproducible build should know it is not.
pub(crate) fn now_secs() -> Result<u64, Unpinned> {
    moment(
        std::env::var_os("SOURCE_DATE_EPOCH").as_deref(),
        std::time::SystemTime::now(),
    )
}

/// `SOURCE_DATE_EPOCH` that does not pin the time, and the time used
/// instead.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Unpinned {
    /// The seconds of now.
    pub(crate) now: u64,
    /// What to tell the author.
    pub(crate) why: String,
}

/// The seconds since 1970 that `epoch` names, space around it allowed;
/// without `epoch`, or with an empty one, those of `now` (none before
/// 1970).
///
/// # Errors
///
/// [`Unpinned`] with those of `now` when `epoch` is not a whole number.
fn moment(epoch: Option<&std::ffi::OsStr>, now: std::time::SystemTime) -> Result<u64, Unpinned> {
    let clock = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let Some(set) = epoch.filter(|value| !value.as_encoded_bytes().trim_ascii().is_empty()) else {
        return Ok(clock);
    };
    set.to_str()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .ok_or_else(|| Unpinned {
            now: clock,
            why: format!(
                "SOURCE_DATE_EPOCH={} is not a whole number of seconds since 1970; the time of the run is used, so the output is not reproducible",
                std::path::Path::new(set).display()
            ),
        })
}

/// Bytes as lowercase hexadecimal, as a fingerprint is written.
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        put!(out, "{byte:02x}");
    }
    out
}

/// Whether `path` lies inside one of `dirs` or the folder md2html runs
/// in, links resolved: what a document may make md2html read.
pub(crate) fn inside(path: &std::path::Path, dirs: &[&std::path::Path]) -> bool {
    let Ok(real) = std::fs::canonicalize(path) else {
        return false;
    };
    let cwd = std::env::current_dir().ok();
    dirs.iter()
        .copied()
        .chain(cwd.as_deref())
        .filter_map(|dir| std::fs::canonicalize(dir).ok())
        .any(|dir| real.starts_with(dir))
}

/// The bytes a URL's text stands for.
///
/// Each `%XX` escape is decoded, everything else is kept as it is. A `%`
/// that is not followed by two hexadecimal digits stays a `%`. The text is
/// read byte by byte, so nothing in it can split a character.
pub(crate) fn percent_decode(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&first, after)) = rest.split_first() {
        rest = after;
        if first == b'%' {
            if let [high_digit, low_digit, tail @ ..] = after {
                if let (Some(high), Some(low)) = (hex_value(*high_digit), hex_value(*low_digit)) {
                    out.push(high * 16 + low);
                    rest = tail;
                    continue;
                }
            }
        }
        out.push(first);
    }
    out
}

/// The value of a hexadecimal digit, either case.
const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// A URL path with its `%XX` escapes decoded, as a name on disk; none
/// when the bytes it decodes to are not UTF-8, as no file md2html can
/// look up portably has such a name.
pub(crate) fn percent_decode_path(path: &str) -> Option<String> {
    String::from_utf8(percent_decode(path)).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        UrlFor, esc, esc_into, hex, is_relative, json_str, json_str_into, moment, now_secs,
        percent_decode, percent_decode_path, plain, safe_url, slug, url_attr,
    };

    /// Each `%XX` escape becomes its byte, and anything that is not one is kept
    /// as written.
    ///
    /// # Panics
    ///
    /// Panics when an escape in either case is not decoded to its byte (`%ff`
    /// and `%00` included), when `%2541` is decoded twice, when a `%` without
    /// two hexadecimal digits after it is not kept as it is, when a character
    /// after `%` is cut, when an escaped UTF-8 path is not decoded to its
    /// characters, or when a path whose escapes are not UTF-8 (a lone byte, a
    /// cut sequence, a surrogate) gives a file name instead of None.
    #[test]
    fn percent_escapes_become_their_bytes() {
        assert_eq!(percent_decode("a%20b"), b"a b");
        assert_eq!(percent_decode("%89PNG%ff"), [0x89, b'P', b'N', b'G', 0xFF]);
        assert_eq!(percent_decode("%00%09%Af%fA%7e"), [
            0x00, 0x09, 0xAF, 0xFA, 0x7E
        ]);
        assert_eq!(percent_decode("%2541"), b"%41");
        // Not an escape: kept as written, whatever follows the `%`.
        for kept in [
            "%", "a%", "%4", "%zz", "%+f", "%-1", "100%", "%g0", "%0g", "%:0", "%/0",
        ] {
            assert_eq!(percent_decode(kept), kept.as_bytes(), "{kept}");
        }
        assert_eq!(percent_decode("%%41"), b"%A");
        // A character after `%` is never cut: the bytes come back whole.
        assert_eq!(percent_decode("%a\u{e9}"), "%a\u{e9}".as_bytes());
        assert_eq!(
            percent_decode_path("%C3%A9t%C3%A9.png").as_deref(),
            Some("\u{e9}t\u{e9}.png")
        );
        // Bytes that are not UTF-8 name no file to look for.
        for not_utf8 in [
            "%ff.png",
            "%C3.png",
            "%C3%28",
            "a/%80/b.png",
            "%ED%A0%80.png",
        ] {
            assert_eq!(percent_decode_path(not_utf8), None, "{not_utf8}");
        }
    }

    /// A script URL is never safe, however it is written, and `data:` is safe
    /// only for images where images are asked for.
    ///
    /// # Panics
    ///
    /// Panics when `javascript:` is let through in any case, with space before
    /// its colon or with a tab or a NUL inside the word; when `vbscript:`,
    /// `livescript:` or `mocha:` is let through; when a `data:` URL is let
    /// through where no image is asked for, or one that names an image only
    /// after another type; or when a `data:` image where one is asked for, a
    /// web address, a relative path with a colon in it, a query that holds
    /// `javascript:`, a fragment, an empty URL or a `sandbox:` path is refused.
    #[test]
    fn script_urls_are_not_links() {
        assert!(!safe_url("javascript:alert(1)", UrlFor::Link));
        assert!(!safe_url(" JaVaScRiPt :alert(1)", UrlFor::Link));
        assert!(!safe_url("java\tscript:alert(1)", UrlFor::Link));
        assert!(!safe_url("java\u{0}script:alert(1)", UrlFor::Link));
        for scheme in ["vbscript", "livescript", "mocha"] {
            assert!(!safe_url(&format!("{scheme}:x"), UrlFor::Image), "{scheme}");
        }
        assert!(!safe_url("data:text/html,hi", UrlFor::Link));
        assert!(!safe_url("data:image/png;base64,AAAA", UrlFor::Link));
        assert!(!safe_url("data:text/html;image/png,x", UrlFor::Image));
        assert!(safe_url("data:image/png;base64,AAAA", UrlFor::Image));
        assert!(safe_url("DATA:Image/SVG+xml,<svg/>", UrlFor::Image));
        assert!(safe_url("https://example.com", UrlFor::Link));
        assert!(safe_url("notes/a:b.md", UrlFor::Link));
        assert!(safe_url("page?x=javascript:1", UrlFor::Link));
        assert!(safe_url("#top", UrlFor::Link));
        assert!(safe_url("", UrlFor::Link));
        assert!(safe_url("sandbox:/mnt/data/x", UrlFor::Link));
    }

    /// A link is relative when it has no scheme and leaves the page.
    ///
    /// # Panics
    ///
    /// Panics when a file name, a path, a path climbing with `..`, one with a
    /// colon after its first `/`, `?` or `#`, or a path from the root is not
    /// taken as relative; or when a fragment, a link without a scheme to
    /// another host, a web address, `mailto:` or `c:a` is.
    #[test]
    fn relative_means_no_scheme_and_not_in_the_page() {
        for relative in [
            "a.md", "dir/a.md", "../a.png", "a/b:c.md", "a?q=x:y", "a#x:y", "/abs",
        ] {
            assert!(is_relative(relative), "{relative}");
        }
        for not in ["#top", "//host/a.png", "https://e.org", "mailto:x@y", "c:a"] {
            assert!(!is_relative(not), "{not}");
        }
    }

    /// A URL written in an attribute has what a URL may not hold
    /// percent-encoded, and `&` escaped.
    ///
    /// # Panics
    ///
    /// Panics when a space, `é` or `"` is not percent-encoded, when `&` is not
    /// written `&amp;`, when an existing `%41` is encoded again, when `<` and
    /// `>` are not encoded, or when any character a URL may hold is changed.
    #[test]
    fn urls_are_escaped_for_an_attribute() {
        assert_eq!(
            url_attr("a b/\u{e9}?x=1&y=\"2\""),
            "a%20b/%C3%A9?x=1&amp;y=%222%22"
        );
        assert_eq!(url_attr("%41<>"), "%41%3C%3E");
        assert_eq!(url_attr("-._~:/?#@!$'()*+,;="), "-._~:/?#@!$'()*+,;=");
    }

    /// Text is escaped for HTML, and HTML is read back as its words.
    ///
    /// # Panics
    ///
    /// Panics when `<`, `>`, `"` and `&` are not escaped or `'` is, or when
    /// HTML read back does not give its words with one space between table
    /// cells, paragraphs, line breaks and inline tags, and `&amp;` decoded.
    #[test]
    fn text_is_escaped_and_html_read_back_as_words() {
        assert_eq!(
            esc("<a href=\"x\">&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;'&lt;/a&gt;"
        );
        assert_eq!(
            plain("<table><tr><td>one</td><td>two</td></tr></table><p>a&amp;b</p><br>c <em>d</em>"),
            "one two a&b c d"
        );
    }

    /// Text escaped into the page is escaped as [`esc`] escapes it, after
    /// what the page holds already.
    ///
    /// # Panics
    ///
    /// Panics when `esc_into` writes anything but what `esc` gives, or
    /// changes what was written before it.
    #[test]
    fn text_escaped_into_the_page_is_escaped_the_same() {
        let text = "<script>alert(\"x\")</script> & 'é'";
        let mut page = String::from("<p>");
        esc_into(&mut page, text);
        assert_eq!(page, format!("<p>{}", esc(text)));
        assert!(!page.contains("<script"));
    }

    /// HTML read back as words takes a tag's name from the tag however
    /// long the tag is, and leaves the tag and all of its attributes out.
    ///
    /// # Panics
    ///
    /// Panics when an attribute's value (a long data URL, a `<` in it)
    /// shows among the words, a cell-ending tag with attributes does not
    /// end a word, or a tag left open at the end lets its text through.
    #[test]
    fn a_tag_is_read_by_its_name_alone() {
        let image = format!(
            "<img src=\"data:image/png;base64,{}\" alt=\"a<b\">",
            "A".repeat(4096)
        );
        assert_eq!(plain(&format!("<p>one{image}two</p>")), "onetwo");
        assert_eq!(plain("<td class=\"x\">one</td\n><td>two</td>"), "one two");
        assert_eq!(plain("one <span title=\"never closed"), "one");
    }

    /// Bytes are written as two lower-case hexadecimal digits each.
    ///
    /// # Panics
    ///
    /// Panics when a byte loses its leading zero, a letter digit is upper case,
    /// or no bytes give anything but an empty text.
    #[test]
    fn bytes_are_written_as_lowercase_hexadecimal() {
        assert_eq!(hex(&[0x00, 0x0F, 0xA5, 0xFF]), "000fa5ff");
        assert_eq!(hex(&[]), "");
    }

    /// The time is `SOURCE_DATE_EPOCH` when it holds a number, and the clock
    /// when it is not set; one that is set to something else is reported.
    ///
    /// # Panics
    ///
    /// Panics when a pinned number with space around it is not taken, when no
    /// pin or an empty one does not give the clock, when a pin that is not a
    /// number is not refused with the clock and a message naming it, when the
    /// start of 1970 is not 0, or when `now_secs` differs by more than two
    /// seconds from the time read the same way in the test.
    #[test]
    fn the_time_is_the_pinned_one_or_the_clock() {
        let then = std::time::UNIX_EPOCH + core::time::Duration::from_secs(1_234);
        let pinned = std::ffi::OsStr::new(" 1700000000\n");
        assert_eq!(moment(Some(pinned), then), Ok(1_700_000_000));
        assert_eq!(moment(None, then), Ok(1_234));
        assert_eq!(moment(Some(std::ffi::OsStr::new(" ")), then), Ok(1_234));
        assert_eq!(moment(None, std::time::UNIX_EPOCH), Ok(0));
        let refused = moment(Some(std::ffi::OsStr::new("soon")), then).unwrap_err();
        assert_eq!(refused.now, 1_234);
        assert!(
            refused
                .why
                .starts_with("SOURCE_DATE_EPOCH=soon is not a whole number of seconds"),
            "{}",
            refused.why
        );
        let before = std::time::SystemTime::now();
        let now = now_secs().unwrap_or_else(|unpinned| unpinned.now);
        let pinned_now = moment(std::env::var_os("SOURCE_DATE_EPOCH").as_deref(), before)
            .unwrap_or_else(|unpinned| unpinned.now);
        assert!(now.abs_diff(pinned_now) <= 2, "{now} {pinned_now}");
    }

    /// A heading's slug is lower case with hyphens, and keeps letters beyond
    /// ASCII.
    ///
    /// # Panics
    ///
    /// Panics when `Über` loses its `ü` or its case is kept, when spaces and
    /// punctuation do not become single hyphens, or when hyphens are left at
    /// either end.
    #[test]
    fn slugs_keep_letters_beyond_ascii() {
        assert_eq!(slug("3. Über die Lage"), "3-über-die-lage");
        assert_eq!(slug("What a document can use"), "what-a-document-can-use");
        assert_eq!(slug("--A  b--"), "a-b");
    }

    /// A JSON literal cannot end the script it stands in, whatever the text
    /// holds.
    ///
    /// # Panics
    ///
    /// Panics when a `<` is written as itself (so `</script>` could close
    /// the page's script), when a quote or a backslash is not escaped, when
    /// a line or paragraph separator is written raw (a line end in older
    /// JavaScript), when a line break or tab is not a space, or when another
    /// control character is kept.
    #[test]
    fn json_literals_cannot_close_a_script() {
        assert_eq!(
            json_str("</script><script>alert(1)</script>"),
            "\"\\u003c/script>\\u003cscript>alert(1)\\u003c/script>\""
        );
        assert_eq!(json_str("say \"hi\" \\ bye"), "\"say \\\"hi\\\" \\\\ bye\"");
        assert_eq!(json_str("a\u{2028}b\u{2029}c"), "\"a\\u2028b\\u2029c\"");
        assert_eq!(json_str("one\ntwo\r\tthree"), "\"one two  three\"");
        assert_eq!(json_str("ring\u{7}\u{0} done"), "\"ring done\"");
        assert_eq!(json_str("\u{e9}t\u{e9}"), "\"\u{e9}t\u{e9}\"");
        let mut index = String::from("[");
        json_str_into(&mut index, "</script>");
        assert_eq!(index, "[\"\\u003c/script>\"");
    }
}
