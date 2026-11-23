// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading text with a limit.
//!
//! What md2html reads is read through a limit of its size, never by asking
//! the file system how large a file is and trusting the answer: a pipe, a
//! device such as `/dev/zero` behind a symlink, or a file that grows while
//! it is read all report a size they do not keep to.

use std::io::Read;
use std::path::Path;

/// The largest file a document makes md2html read beside it: a
/// stylesheet, a head, mermaid.js, the original of a translation, the
/// shared glossary.
pub(crate) const MAX_BESIDE: u64 = 64 << 20;

/// Why text could not be read.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unread {
    /// It is larger than the limit, in bytes.
    TooLarge(u64),
    /// It is not UTF-8.
    NotText,
    /// Reading failed.
    Failed(String),
}

impl Unread {
    /// What to tell the person, after the name of what was read.
    pub(crate) fn describe(&self, raise: &str) -> String {
        match self {
            Self::TooLarge(limit) => format!("larger than {} MB{raise}", limit >> 20_i32),
            Self::NotText => String::from("stream did not contain valid UTF-8"),
            Self::Failed(error) => error.clone(),
        }
    }
}

/// All of `source` as text, refused when it holds more than `max` bytes.
///
/// # Errors
///
/// It holds more than `max` bytes, is not UTF-8, or cannot be read.
pub(crate) fn read_limited(source: impl Read, max: u64) -> Result<String, Unread> {
    String::from_utf8(capped(source, max)?)
        .ok()
        .ok_or(Unread::NotText)
}

/// The file at `path` as text, refused when it holds more than `max` bytes.
///
/// # Errors
///
/// It cannot be opened, or [`read_limited`] refuses it.
pub(crate) fn read_file(path: &Path, max: u64) -> Result<String, Unread> {
    read_limited(open(path)?, max)
}

/// The file at `path` as bytes, refused when it holds more than `max`.
///
/// # Errors
///
/// It cannot be opened or read, or holds more than `max` bytes.
pub(crate) fn read_bytes(path: &Path, max: u64) -> Result<Vec<u8>, Unread> {
    capped(open(path)?, max)
}

/// The file at `path`, open for reading.
fn open(path: &Path) -> Result<std::fs::File, Unread> {
    std::fs::File::open(path).map_err(|error| Unread::Failed(error.to_string()))
}

/// All of `source`, reading at most one byte past `max` to know it is
/// too large.
fn capped(source: impl Read, max: u64) -> Result<Vec<u8>, Unread> {
    let mut bytes = Vec::new();
    source
        .take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| Unread::Failed(error.to_string()))?;
    if u64::try_from(bytes.len()).is_ok_and(|read| read > max) {
        return Err(Unread::TooLarge(max));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{Unread, read_limited};

    #[test]
    fn text_is_read_up_to_its_limit_and_no_further() {
        assert_eq!(
            read_limited(b"four".as_slice(), 4),
            Ok(String::from("four"))
        );
        assert_eq!(
            read_limited(b"five!".as_slice(), 4),
            Err(Unread::TooLarge(4))
        );
        // An endless source is read only one byte past the limit.
        assert_eq!(
            read_limited(std::io::repeat(b'a'), 1 << 20),
            Err(Unread::TooLarge(1 << 20))
        );
        assert_eq!(read_limited(b"\xff".as_slice(), 4), Err(Unread::NotText));
        assert_eq!(
            Unread::TooLarge(64 << 20).describe(" (--max-input raises the limit)"),
            "larger than 64 MB (--max-input raises the limit)"
        );
    }
}
