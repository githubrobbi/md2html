// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading text with a limit.
//!
//! What md2html reads is read through a limit of its size, never by asking
//! the file system how large a file is and trusting the answer: a pipe, a
//! device such as `/dev/zero` behind a symlink, or a file that grows while
//! it is read all report a size they do not keep to.
//!
//! What the person names on the command line is read whatever it is, a
//! pipe included (`md2html <(...)`): [`read_file`]. What a document names
//! (a stylesheet, a head, the original of a translation, the shared
//! glossary, an image) is read only when it is a file: [`read_named`] and
//! [`read_named_bytes`]. Opening a named pipe waits until something writes
//! to it, so a document that named one would stall md2html for ever.

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
    /// It is not there: the file system's reason.
    Missing(String),
    /// It is there, but not a file: a folder, a pipe, a device.
    NotAFile,
    /// Reading failed.
    Failed(String),
}

impl Unread {
    /// What to tell the person, after the name of what was read.
    pub(crate) fn describe(&self, raise: &str) -> String {
        match self {
            Self::TooLarge(limit) => format!("larger than {} MB{raise}", limit >> 20_i32),
            Self::NotText => String::from("stream did not contain valid UTF-8"),
            Self::NotAFile => String::from("it is not a file"),
            Self::Missing(error) | Self::Failed(error) => error.clone(),
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

/// A file a document names, as text, refused when it holds more than
/// `max` bytes or is not a file.
///
/// # Errors
///
/// [`Unread::NotAFile`] when `path` is a folder, a pipe or a device;
/// otherwise as [`read_file`].
pub(crate) fn read_named(path: &Path, max: u64) -> Result<String, Unread> {
    read_limited(open_named(path)?, max)
}

/// A file a document names, as bytes, refused when it holds more than
/// `max` bytes or is not a file.
///
/// # Errors
///
/// [`Unread::NotAFile`] when `path` is a folder, a pipe or a device; it
/// cannot be opened or read, or holds more than `max` bytes.
pub(crate) fn read_named_bytes(path: &Path, max: u64) -> Result<Vec<u8>, Unread> {
    capped(open_named(path)?, max)
}

/// The file at `path`, open for reading, when it is a file (links
/// followed).
///
/// Asked before it is opened, because opening a named pipe waits for a
/// writer. Something that turns into a pipe between the question and the
/// opening is not caught; that takes someone who can write in the
/// document's folder while md2html runs.
///
/// # Errors
///
/// [`Unread::NotAFile`] when it is there but not a file; otherwise as
/// [`open`].
fn open_named(path: &Path) -> Result<std::fs::File, Unread> {
    match std::fs::metadata(path) {
        Ok(found) if !found.is_file() => Err(Unread::NotAFile),
        Ok(_) | Err(_) => open(path),
    }
}

/// The file at `path`, open for reading.
///
/// # Errors
///
/// With the file system's reason: [`Unread::Missing`] when the file does
/// not exist, [`Unread::Failed`] when it may not be read.
fn open(path: &Path) -> Result<std::fs::File, Unread> {
    std::fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Unread::Missing(error.to_string())
        } else {
            Unread::Failed(error.to_string())
        }
    })
}

/// All of `source`, reading at most one byte past `max` to know it is
/// too large.
///
/// # Errors
///
/// [`Unread::TooLarge`] with `max` when `source` holds more than `max`
/// bytes, and [`Unread::Failed`] with the reason when reading fails.
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
    use super::{Unread, read_limited, read_named, read_named_bytes};

    /// What a document names is read only when it is a file: a folder is
    /// refused, and so is a named pipe, which opening would wait on for
    /// ever.
    ///
    /// # Panics
    ///
    /// When a folder or a pipe is not refused as not a file (or the test
    /// hangs on the pipe), when a file beside them is not read, or when the
    /// test's folder or pipe cannot be made or removed.
    #[test]
    fn only_a_file_is_read_when_a_document_names_it() {
        let dir = std::env::temp_dir().join(format!("md2html-named-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("folder.css")).unwrap();
        std::fs::write(dir.join("style.css"), "p{}").unwrap();
        assert_eq!(
            read_named(&dir.join("folder.css"), 64),
            Err(Unread::NotAFile)
        );
        assert_eq!(
            read_named(&dir.join("style.css"), 64),
            Ok(String::from("p{}"))
        );
        assert!(matches!(
            read_named(&dir.join("missing.css"), 64),
            Err(Unread::Missing(_))
        ));
        if cfg!(unix) {
            let made = std::process::Command::new("mkfifo")
                .arg(dir.join("pipe.css"))
                .status()
                .unwrap();
            assert!(made.success());
            assert_eq!(
                read_named_bytes(&dir.join("pipe.css"), 64),
                Err(Unread::NotAFile)
            );
        }
        assert_eq!(Unread::NotAFile.describe(""), "it is not a file");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Text is read up to its limit, and an endless source no further.
    ///
    /// # Panics
    ///
    /// When text of exactly the limit is refused, when one byte more is read,
    /// when an endless source is not refused as too large (so reading it would
    /// never end), when bytes that are not UTF-8 are not refused as not text,
    /// or when the size message does not give the limit in MB and the advice
    /// after it.
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
