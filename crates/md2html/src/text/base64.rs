// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Base64 (RFC 4648): written for embedded images and hashes, read back
//! from `data:` URLs.

use super::runs;

/// Standard base64, for embedded images.
pub(crate) fn base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    base64_into(&mut out, bytes);
    out
}

/// Writes standard base64 to `out`: [`base64`] after what `out` holds,
/// an embedded image's data URL written in one string.
pub(crate) fn base64_into(out: &mut String, bytes: &[u8]) {
    out.reserve(bytes.len().div_ceil(3) * 4);
    let mut rest = bytes;
    // A line of digits at a time: written into an array of bytes, then
    // added to the string at once, not one character after another.
    for run in runs::<LINE_BYTES>(&mut rest) {
        let mut line = [0_u8; LINE_BYTES / 3 * 4];
        for (slots, group) in line.chunks_exact_mut(4).zip(runs::<3>(&mut run.as_slice())) {
            for (slot, digit) in slots.iter_mut().zip(quad(*group)) {
                *slot = digit;
            }
        }
        // The digits are ASCII, so the line is text as it is; were a byte
        // not, it would be written as the character of its value.
        match core::str::from_utf8(&line) {
            Ok(digits) => out.push_str(digits),
            Err(_) => out.extend(line.map(char::from)),
        }
    }
    for group in runs::<3>(&mut rest) {
        put_digits(out, quad(*group), 4);
    }
    match *rest {
        [first] => put_digits(out, quad([first, 0, 0]), 2),
        [first, second] => put_digits(out, quad([first, second, 0]), 3),
        _ => {}
    }
}

/// How many bytes are written as one line of digits.
const LINE_BYTES: usize = 768;

/// The first `digits` of four, the rest padded with `=`.
fn put_digits(out: &mut String, quad: [u8; 4], digits: usize) {
    out.extend(quad.into_iter().take(digits).map(char::from));
    out.extend(core::iter::repeat_n('=', 4 - digits));
}

/// Three bytes as the four digits of their four six-bit values.
fn quad(group: [u8; 3]) -> [u8; 4] {
    let [first, second, third] = group;
    let value = u32::from_be_bytes([0, first, second, third]);
    [18_u32, 12, 6, 0].map(|shift| {
        let [.., low] = (value >> shift).to_be_bytes();
        digit(low % 64)
    })
}

/// The digit of a six-bit value: `A` to `Z`, `a` to `z`, `0` to `9`, `+`
/// and `/`. Worked out from the value without a branch: the digits of
/// random bytes (an image) jump between the five ranges unpredictably.
fn digit(sextet: u8) -> u8 {
    let mut digit = sextet.wrapping_add(b'A');
    // From `a`: past the 26 capitals, `a` is 6 after where `Z` + 1 is.
    digit = digit.wrapping_add(6 * past(sextet, 26));
    // From `0`, at 52.
    digit = digit.wrapping_sub(75 * past(sextet, 52));
    // `+` at 62, then `/` at 63.
    digit = digit.wrapping_sub(15 * past(sextet, 62));
    digit.wrapping_add(3 * past(sextet, 63))
}

/// 1 when `value` is `start` or more, 0 before.
fn past(value: u8, start: u8) -> u8 {
    u8::from(value >= start)
}

/// The six-bit value of a character: `+` and `/`, or `-` and `_` as the
/// URL-safe alphabet writes them. None for anything else.
const fn sextet(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

/// Standard or URL-safe base64 read back; padding and white space are
/// passed over. None when it is not base64.
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    // The bits read and not yet written, as a number below 2^`bits`.
    let (mut pending, mut bits) = (0_u32, 0_u32);
    for byte in text.bytes() {
        if matches!(byte, b'=' | b' ' | b'\n' | b'\r' | b'\t') {
            continue;
        }
        pending = pending * 64 + u32::from(sextet(byte)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            let [.., written] = (pending >> bits).to_be_bytes();
            out.push(written);
            pending %= 1 << bits;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{LINE_BYTES, base64, base64_decode, base64_into, digit, sextet};

    /// The alphabet of RFC 4648, table 1: the digit of each six-bit value.
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    /// Base64 as the RFC describes it, one group of three bytes after
    /// another, each six-bit value looked up in the alphabet.
    ///
    /// # Panics
    ///
    /// Only were a six-bit value past the alphabet's 64 digits.
    fn by_the_book(bytes: &[u8]) -> String {
        let mut out = String::new();
        for group in bytes.chunks(3) {
            let mut padded = [0_u8; 3];
            padded
                .iter_mut()
                .zip(group)
                .for_each(|(slot, byte)| *slot = *byte);
            let [first, second, third] = padded;
            let value = u32::from_be_bytes([0, first, second, third]);
            for (place, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
                if place <= group.len() {
                    let index = usize::try_from((value >> shift) & 63).expect("six bits");
                    out.push(ALPHABET.chars().nth(index).expect("a digit"));
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    /// Every six-bit value has the digit the alphabet gives it.
    ///
    /// # Panics
    ///
    /// When any of the 64 values is written as another digit than the
    /// RFC's alphabet has in its place, or its digit is not read back as it.
    #[test]
    fn each_value_has_the_alphabets_digit() {
        for (value, expected) in (0_u8..64).zip(ALPHABET.chars()) {
            assert_eq!(char::from(digit(value)), expected, "{value}");
            assert_eq!(sextet(digit(value)), Some(value), "{value}");
        }
    }

    /// Long inputs, written a line at a time, are the RFC's base64 to the
    /// byte, at every length around the end of a line.
    ///
    /// # Panics
    ///
    /// When any input of 0 to three lines and a few bytes, of every byte
    /// value, is encoded otherwise than one group at a time with the
    /// alphabet, or is not decoded back to itself.
    #[test]
    fn lines_of_digits_are_the_groups_written_one_by_one() {
        let bytes: Vec<u8> = (0..3 * LINE_BYTES + 7)
            .map(|place| u8::try_from((place * 167 + place / 256) % 256).expect("a byte"))
            .collect();
        for len in (0..=bytes.len()).filter(|len| {
            len % LINE_BYTES < 8 || len % LINE_BYTES > LINE_BYTES - 8 || len % 97 == 0
        }) {
            let input = bytes.get(..len).expect("a beginning of the bytes");
            let coded = base64(input);
            assert_eq!(coded, by_the_book(input), "{len}");
            assert_eq!(base64_decode(&coded).as_deref(), Some(input), "{len}");
        }
    }

    /// The test vectors of RFC 4648, section 10.
    const VECTORS: [(&str, &str); 7] = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];

    /// The test vectors of RFC 4648 encode and decode both ways.
    ///
    /// # Panics
    ///
    /// Panics when any of the seven vectors, from the empty text to `foobar`,
    /// is not encoded to its standard form with its `=` padding, or is not
    /// decoded back to the same bytes; when `Man` is not `TWFu`; or when
    /// base64 written after a data URL's head is not what it would be alone.
    #[test]
    fn the_standard_vectors_go_both_ways() {
        for (plain, coded) in VECTORS {
            assert_eq!(base64(plain.as_bytes()), coded, "{plain}");
            assert_eq!(
                base64_decode(coded).as_deref(),
                Some(plain.as_bytes()),
                "{coded}"
            );
        }
        assert_eq!(base64(b"Man"), "TWFu");
        let mut url = String::from("data:text/plain;base64,");
        base64_into(&mut url, b"foobar");
        assert_eq!(url, "data:text/plain;base64,Zm9vYmFy");
    }

    /// Each of the 64 digits stands for its own value, and every byte value
    /// survives the round trip.
    ///
    /// # Panics
    ///
    /// Panics when the encoding of the bytes 0 to 255 does not start and end as
    /// it should or does not decode back to them, when bytes chosen to give the
    /// digits `A` to `L`, `+` or `/` give others, or when a text without its
    /// padding is not decoded.
    #[test]
    fn every_digit_stands_for_its_value() {
        let every: Vec<u8> = (0..=255).collect();
        let coded = base64(&every);
        assert!(coded.starts_with("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIj"));
        assert!(coded.ends_with("8PHy8/T19vf4+fr7/P3+/w=="));
        assert_eq!(base64_decode(&coded), Some(every));
        assert_eq!(
            base64(&[0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B]),
            "ABCDEFGHIJKL"
        );
        assert_eq!(base64(&[0xFB, 0xEF, 0xBE]), "++++");
        assert_eq!(base64(&[0xFF, 0xFF, 0xFF]), "////");
        assert_eq!(
            base64_decode("MDEyMzQ1Njc4OQ"),
            Some(b"0123456789".to_vec())
        );
    }

    /// The decoder reads the URL-safe digits and skips white space, and refuses
    /// anything else.
    ///
    /// # Panics
    ///
    /// Panics when `-` and `_` are not read as `+` and `/`, when spaces, tabs
    /// or line breaks inside the text are not skipped, when a lone digit at the
    /// end does not decode to nothing more, or when `!`, `é`, `.` or `*` is
    /// decoded instead of refused.
    #[test]
    fn the_url_safe_alphabet_and_white_space_are_read_too() {
        assert_eq!(base64_decode("-_8="), Some(vec![0xFB, 0xFF]));
        assert_eq!(base64_decode("+/8="), Some(vec![0xFB, 0xFF]));
        assert_eq!(base64_decode(" Zm9v\r\nYm\tFy\n"), Some(b"foobar".to_vec()));
        // A lone character holds too few bits for a byte.
        assert_eq!(base64_decode("Zm9vY"), Some(b"foo".to_vec()));
        for bad in ["Zm9v!", "Zm\u{e9}9v", "Zm9v.", "*"] {
            assert_eq!(base64_decode(bad), None, "{bad}");
        }
    }
}
