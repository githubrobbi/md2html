// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::default_numeric_fallback,
    clippy::indexing_slicing,
    clippy::min_ident_chars,
    clippy::missing_asserts_for_indexing,
    clippy::redundant_type_annotations,
    clippy::shadow_reuse,
    clippy::string_slice,
    clippy::unseparated_literal_suffix,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]
#![cfg_attr(
    test,
    expect(
        clippy::let_underscore_must_use,
        reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
    )
)]

//! Small text helpers every part of the page uses: escaping, anchors,
//! HTML back to words, URLs, and the date.

/// Escapes text for HTML, attribute values included.
pub(crate) fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
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
    let mut tag: Option<String> = None;
    for ch in html.chars() {
        match (ch, tag.as_mut()) {
            ('<', None) => tag = Some(String::new()),
            ('>', Some(name)) => {
                let head = name.split_whitespace().next().unwrap_or_default();
                if BREAKS.contains(&head) {
                    out.push(' ');
                }
                tag = None;
            }
            (other, Some(name)) => name.push(other),
            (other, None) => out.push(other),
        }
    }
    crate::inline::decode_entities(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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
            let mut buf = [0u8; 4];
            for byte in ch.encode_utf8(&mut buf).bytes() {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 15)]));
            }
        }
    }
    esc(&out)
}

/// Whether a link or image destination may be written into the page.
/// Script URLs never; `data:` only for an image's pixels.
pub(crate) fn safe_url(url: &str, image: bool) -> bool {
    let cleaned: String = url
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_control())
        .collect::<String>()
        .to_ascii_lowercase();
    let Some(colon) = cleaned.find(':') else {
        return true;
    };
    let scheme = &cleaned[..colon];
    if scheme.contains(['/', '?', '#']) {
        return true;
    }
    match scheme {
        "javascript" | "vbscript" | "livescript" | "mocha" => false,
        "data" => {
            image
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

/// Whether a destination points into the page or the file system next to
/// it rather than somewhere with a scheme.
pub(crate) fn is_relative(url: &str) -> bool {
    !url.starts_with('#')
        && !url.starts_with("//")
        && url
            .find(':')
            .is_none_or(|colon| url[..colon].contains(['/', '?', '#']))
}

/// The civil date of `secs` since the epoch: (year, month, day).
pub(crate) fn civil(secs: u64) -> (i64, u32, u32) {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let shifted = days.saturating_add(719_468);
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

/// Now, or `SOURCE_DATE_EPOCH` when it is set, so a build can be
/// reproduced byte for byte.
pub(crate) fn now_secs() -> u64 {
    if let Some(epoch) = std::env::var_os("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = epoch.to_string_lossy().trim().parse::<u64>() {
            return secs;
        }
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// A `YYYY-MM-DD` date: (year, month, day).
pub(crate) fn iso_date(text: &str) -> Option<(i64, u32, u32)> {
    let mut parts = text.trim().splitn(3, '-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts
        .next()?
        .parse::<u32>()
        .ok()
        .filter(|m| (1..=12).contains(m))?;
    let day: String = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let day = day.parse::<u32>().ok().filter(|d| (1..=31).contains(d))?;
    Some((year, month, day))
}

/// Bytes as lowercase hexadecimal, as a fingerprint is written.
pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| {
            [
                char::from(DIGITS[usize::from(b >> 4)]),
                char::from(DIGITS[usize::from(b & 15)]),
            ]
        })
        .collect()
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
        .filter_map(|d| std::fs::canonicalize(d).ok())
        .any(|d| real.starts_with(d))
}

/// The bytes a URL's text stands for: each `%XX` escape decoded, everything
/// else as it is. A `%` that is not followed by two hexadecimal digits stays a
/// `%`. The text is read byte by byte, so nothing in it can split a character.
pub(crate) fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// A URL path with its `%XX` escapes decoded, as a name on disk.
pub(crate) fn percent_decode_path(path: &str) -> String {
    // AUDIT-OK(bytes): the result is only ever a file name to look for. Bytes
    // that are not UTF-8 become U+FFFD, which names no file the document could
    // not already name, and whether a file may be read is decided afterwards,
    // on the path the file system resolves (`inside`), never on this text.
    String::from_utf8_lossy(&percent_decode(path)).into_owned()
}

/// Standard base64 read back; None when it is not base64.
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for byte in text.bytes() {
        let v = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xFF).unwrap_or(0));
        }
    }
    Some(out)
}

/// Standard base64, for embedded images.
pub(crate) fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ABC[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// SHA-256 of `data`, for the page's Content-Security-Policy hashes.
#[expect(
    clippy::too_many_lines,
    reason = "one dispatch over the kinds it handles, read top to bottom"
)]
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428A_2F98,
        0x7137_4491,
        0xB5C0_FBCF,
        0xE9B5_DBA5,
        0x3956_C25B,
        0x59F1_11F1,
        0x923F_82A4,
        0xAB1C_5ED5,
        0xD807_AA98,
        0x1283_5B01,
        0x2431_85BE,
        0x550C_7DC3,
        0x72BE_5D74,
        0x80DE_B1FE,
        0x9BDC_06A7,
        0xC19B_F174,
        0xE49B_69C1,
        0xEFBE_4786,
        0x0FC1_9DC6,
        0x240C_A1CC,
        0x2DE9_2C6F,
        0x4A74_84AA,
        0x5CB0_A9DC,
        0x76F9_88DA,
        0x983E_5152,
        0xA831_C66D,
        0xB003_27C8,
        0xBF59_7FC7,
        0xC6E0_0BF3,
        0xD5A7_9147,
        0x06CA_6351,
        0x1429_2967,
        0x27B7_0A85,
        0x2E1B_2138,
        0x4D2C_6DFC,
        0x5338_0D13,
        0x650A_7354,
        0x766A_0ABB,
        0x81C2_C92E,
        0x9272_2C85,
        0xA2BF_E8A1,
        0xA81A_664B,
        0xC24B_8B70,
        0xC76C_51A3,
        0xD192_E819,
        0xD699_0624,
        0xF40E_3585,
        0x106A_A070,
        0x19A4_C116,
        0x1E37_6C08,
        0x2748_774C,
        0x34B0_BCB5,
        0x391C_0CB3,
        0x4ED8_AA4A,
        0x5B9C_CA4F,
        0x682E_6FF3,
        0x748F_82EE,
        0x78A5_636F,
        0x84C8_7814,
        0x8CC7_0208,
        0x90BE_FFFA,
        0xA450_6CEB,
        0xBEF9_A3F7,
        0xC671_78F2,
    ];
    let mut h: [u32; 8] = [
        0x6A09_E667,
        0xBB67_AE85,
        0x3C6E_F372,
        0xA54F_F53A,
        0x510E_527F,
        0x9B05_688C,
        0x1F83_D9AB,
        0x5BE0_CD19,
    ];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (slot, add) in h.iter_mut().zip(v) {
            *slot = slot.wrapping_add(add);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{base64, percent_decode, percent_decode_path, safe_url, sha256, slug};

    #[test]
    fn percent_escapes_become_their_bytes() {
        assert_eq!(percent_decode("a%20b"), b"a b");
        assert_eq!(percent_decode("%89PNG%ff"), [0x89, b'P', b'N', b'G', 0xFF]);
        // Not an escape: kept as written, whatever follows the `%`.
        for kept in ["%", "a%", "%4", "%zz", "%+f", "%-1", "100%"] {
            assert_eq!(percent_decode(kept), kept.as_bytes(), "{kept}");
        }
        // A character after `%` is never cut: the bytes come back whole.
        assert_eq!(percent_decode("%a\u{e9}"), "%a\u{e9}".as_bytes());
        assert_eq!(
            percent_decode_path("%C3%A9t%C3%A9.png"),
            "\u{e9}t\u{e9}.png"
        );
        assert_eq!(percent_decode_path("%ff.png"), "\u{fffd}.png");
    }

    #[test]
    fn script_urls_are_not_links() {
        assert!(!safe_url("javascript:alert(1)", false));
        assert!(!safe_url(" JaVaScRiPt :alert(1)", false));
        assert!(!safe_url("java\tscript:alert(1)", false));
        assert!(!safe_url("data:text/html,hi", false));
        assert!(!safe_url("data:image/png;base64,AAAA", false));
        assert!(safe_url("data:image/png;base64,AAAA", true));
        assert!(safe_url("https://example.com", false));
        assert!(safe_url("notes/a:b.md", false));
        assert!(safe_url("#top", false));
        assert!(safe_url("sandbox:/mnt/data/x", false));
    }

    #[test]
    fn known_digests() {
        let hex = |d: [u8; 32]| {
            d.iter().fold(String::new(), |mut s, b| {
                use core::fmt::Write as _;
                let _ = write!(s, "{b:02x}");
                s
            })
        };
        assert_eq!(
            hex(sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn slugs_keep_letters_beyond_ascii() {
        assert_eq!(slug("3. Über die Lage"), "3-über-die-lage");
        assert_eq!(slug("What a document can use"), "what-a-document-can-use");
    }
}
