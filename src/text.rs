// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

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

/// A URL path with its `%XX` escapes decoded (invalid UTF-8 replaced).
pub(crate) fn percent_decode_path(path: &str) -> String {
    let bytes = path.as_bytes();
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
    String::from_utf8_lossy(&out).into_owned()
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
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
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
    use super::{base64, safe_url, sha256, slug};

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
