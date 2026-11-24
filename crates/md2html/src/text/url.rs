// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! URLs: whether a page may carry one, how one is written into an
//! attribute, and the bytes its escapes stand for.

use super::{Html, esc, put};

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

/// Appends `name` to `out` as one segment of a URL's path, so that the
/// URL names it and nothing else.
///
/// A byte is kept when RFC 3986 lets a segment hold it and it means
/// nothing there: a letter or a digit, `-._~`, the sub-delimiters
/// `!$&'()*+,;=`, and `@`. Every other byte is written `%XX`: `/` (which
/// would end the segment), `?` and `#` (which would end the path), `%`
/// (which would start an escape), `:` (which in a relative URL's first
/// segment would end a scheme), space, quotes, `<>`, `\` and the rest of
/// ASCII's punctuation, control characters, and each byte of a character
/// beyond ASCII. [`percent_decode`] gives back exactly `name`; a name that
/// is not UTF-8 keeps its own bytes.
pub(crate) fn put_segment(out: &mut String, name: &[u8]) {
    for &byte in name {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=@".contains(&byte) {
            out.push(char::from(byte));
        } else {
            put!(out, "%{byte:02X}");
        }
    }
}

/// A URL md2html makes for a file from the file's names, each name one
/// segment of the path.
///
/// Each name is written by [`put_segment`], so that the page links the
/// file whatever its name holds (a space, `#`, `?`, `%`, a character beyond
/// ASCII) and a server reads the same names back with [`percent_decode`].
///
/// It is written into an attribute only HTML-escaped ([`PathUrl::attr`]):
/// escaping it as a URL again would turn the `%` of its escapes into `%25`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PathUrl(String);

impl PathUrl {
    /// The relative URL of `names`, one segment each, joined by `/`: from a
    /// page's folder, `..` for each folder up, the folders down, and the
    /// file (`.` and `..` are kept as they are).
    #[must_use]
    pub(crate) fn relative<'name>(names: impl IntoIterator<Item = &'name [u8]>) -> Self {
        let mut url = Self(String::new());
        for (at, name) in names.into_iter().enumerate() {
            if at > 0 {
                url.0.push('/');
            }
            put_segment(&mut url.0, name);
        }
        url
    }

    /// The relative URL of a file in the page's own folder.
    #[must_use]
    pub(crate) fn name(name: &str) -> Self {
        Self::relative([name.as_bytes()])
    }

    /// The start of a `file:` URL: its scheme and the empty host of this
    /// machine, before a path (or a host, [`PathUrl::host`]).
    #[must_use]
    pub(crate) fn file() -> Self {
        Self(String::from("file://"))
    }

    /// Appends a host: the server of a Windows share, right after `file://`.
    pub(crate) fn host(&mut self, host: &[u8]) {
        put_segment(&mut self.0, host);
    }

    /// Appends `/` and `name` as the next segment of the path.
    pub(crate) fn push(&mut self, name: &[u8]) {
        self.0.push('/');
        put_segment(&mut self.0, name);
    }

    /// Appends `/` and a Windows drive, `C:`: its colon is the URL's own.
    pub(crate) fn drive(&mut self, letter: u8) {
        self.0.push('/');
        put_segment(&mut self.0, &[letter]);
        self.0.push(':');
    }

    /// The URL as text.
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The URL as text, owned.
    #[must_use]
    pub(crate) fn into_string(self) -> String {
        self.0
    }

    /// The URL as an attribute's value: HTML-escaped (`&` as `&amp;`), and
    /// nothing more.
    #[must_use]
    pub(crate) fn attr(&self) -> Html {
        Html::escape(&self.0)
    }
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
        PathUrl, UrlFor, is_relative, percent_decode, percent_decode_path, put_segment, safe_url,
        url_attr,
    };
    use crate::tests::awkward_names;

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

    /// A name is one segment of a URL: every byte is kept only when a
    /// segment may hold it as it is, and decoding gives the name back.
    ///
    /// # Panics
    ///
    /// When any byte, alone as a name, is written with a character a segment
    /// may not hold, or does not decode to itself; when a name a URL would
    /// read otherwise is not written as expected or does not read back; when
    /// a relative URL does not join its names with `/` and keep `..`; or when
    /// an attribute escapes `&` other than once or a `%` again.
    #[test]
    fn a_name_is_one_segment_and_reads_back() {
        for byte in 0..=u8::MAX {
            let mut url = String::new();
            put_segment(&mut url, &[byte]);
            assert!(
                url.bytes().all(
                    |kept| kept.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=@%".contains(&kept)
                ),
                "{byte:#04x}: {url}"
            );
            assert_eq!(percent_decode(&url), [byte], "{url}");
        }
        for awkward in awkward_names() {
            let file = format!("{}.html", awkward.name);
            let url = PathUrl::name(&file);
            assert_eq!(url.as_str(), format!("{}.html", awkward.url), "{file}");
            assert_eq!(
                percent_decode_path(url.as_str()).as_deref(),
                Some(file.as_str())
            );
        }
        assert_eq!(PathUrl::name("c:d.html").as_str(), "c%3Ad.html");
        assert!(is_relative(PathUrl::name("c:d.html").as_str()));
        let up = PathUrl::relative([&b".."[..], b"a b", b"c#d.html"]);
        assert_eq!(up.as_str(), "../a%20b/c%23d.html");
        assert_eq!(
            PathUrl::name("a&b%.html").attr().to_string(),
            "a&amp;b%25.html"
        );
    }
}
