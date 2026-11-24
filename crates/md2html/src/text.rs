// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Small text helpers every part of the page uses: escaping and its
//! reverse (character references, backslash escapes), anchors, HTML back
//! to words, URLs, the date, and the encodings of bytes.

use crate::scan::Scan;

mod base64;
mod date;
mod entity;
mod html;
mod sha256;
mod url;

pub(crate) use self::base64::{base64, base64_decode, base64_into};
pub(crate) use self::date::{Date, civil, iso_date};
pub(crate) use self::entity::{decode_entities, entity, unescape};
pub(crate) use self::html::Html;
pub(crate) use self::sha256::sha256;
pub(crate) use self::url::{
    PathUrl, SafeUrl, UrlFor, is_relative, percent_decode, percent_decode_path, safe_url, url_attr,
};

/// Appends formatted text to a `String`: `write!` without a result to
/// discard at every call. The first argument is the `String` (or a
/// `&mut String`), the rest is what `format!` takes.
///
/// A macro because `format_args!`, which keeps the formatting free of
/// allocation, exists only as a macro; it expands to one call of
/// [`Put::put_fmt`] and hides no control flow:
///
/// ```text
/// let mut out = String::from("#");
/// put!(out, "{:02X}", 255_u8);
/// assert_eq!(out, "#FF");
/// ```
///
/// (md2html is a binary, whose examples rustdoc does not run; the test
/// `put_appends_what_format_writes` runs this one.)
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
    let mut scan = Scan::new(html);
    // Text up to each tag is copied whole; a tag is passed over to its
    // `>` in one search (an embedded image's is the whole image) and only
    // its name is read. A tag left open at the end is not text.
    loop {
        let Some(text) = scan.take_until("<") else {
            out.push_str(scan.take_rest());
            break;
        };
        out.push_str(text);
        scan.bump();
        let Some(tag) = scan.take_until(">") else {
            break;
        };
        scan.bump();
        if tag
            .split_whitespace()
            .next()
            .is_some_and(|head| BREAKS.contains(&head))
        {
            out.push(' ');
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

/// Where `path` really is, links resolved, when that lies inside one of
/// `dirs` or the folder md2html runs in: what a document may make md2html
/// read. None when it is outside them, or not there.
///
/// The caller reads the path this gives, not `path`: a link on the way
/// that is changed after the check cannot lead the read elsewhere, as the
/// path given has no link left in it.
pub(crate) fn inside(
    path: &std::path::Path,
    dirs: &[&std::path::Path],
) -> Option<std::path::PathBuf> {
    let real = std::fs::canonicalize(path).ok()?;
    let cwd = std::env::current_dir().ok();
    dirs.iter()
        .copied()
        .chain(cwd.as_deref())
        .filter_map(|dir| std::fs::canonicalize(dir).ok())
        .any(|dir| real.starts_with(dir))
        .then_some(real)
}

#[cfg(test)]
mod tests {
    use super::{esc, esc_into, hex, json_str, json_str_into, moment, now_secs, plain, slug};

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

    /// [`put!`] appends what `format!` writes, after what is there: the
    /// example of its documentation.
    ///
    /// # Panics
    ///
    /// Panics when the text is not `#FF`, or a `&mut String` takes it
    /// otherwise.
    #[test]
    fn put_appends_what_format_writes() {
        let mut out = String::from("#");
        put!(out, "{:02X}", 255_u8);
        assert_eq!(out, "#FF");
        let borrowed = &mut out;
        put!(borrowed, "{:02x}", 1_u8);
        assert_eq!(out, "#FF01");
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
    /// end a word, a tag left open at the end lets its text through, or a
    /// `>` of the text outside any tag is left out.
    #[test]
    fn a_tag_is_read_by_its_name_alone() {
        let image = format!(
            "<img src=\"data:image/png;base64,{}\" alt=\"a<b\">",
            "A".repeat(4096)
        );
        assert_eq!(plain(&format!("<p>one{image}two</p>")), "onetwo");
        assert_eq!(plain("<td class=\"x\">one</td\n><td>two</td>"), "one two");
        assert_eq!(plain("one <span title=\"never closed"), "one");
        assert_eq!(plain("a > b <i title=\"<\">c</i>"), "a > b c");
        assert_eq!(plain("no tags at all"), "no tags at all");
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
