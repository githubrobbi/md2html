// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The files a document names beside it, and its own settings.
//!
//! Files the document itself names stay inside its folder (or the folder
//! md2html runs in), with the kind of name their job needs: an untrusted
//! Markdown file cannot pull a secret into its page. The command line may
//! name any file.

use alloc::collections::BTreeSet;
use core::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use crate::config::{FrontMatter, Given, Pair, Settings};
use crate::input::{MAX_BESIDE, read_named};
use crate::render::Warning;
use crate::text;

/// The extensions of a Markdown file.
pub(crate) const MARKDOWN: [&str; 4] = ["md", "markdown", "mdown", "mkd"];

/// The settings of the front matter, the files that may not be read taken
/// out; what was wrong, as warnings.
///
/// A key given twice, a setting given by two of its names, a key md2html
/// does not know and a file it may not read are each told.
pub(super) fn own_settings(
    front: &FrontMatter<'_>,
    source: Option<&Path>,
) -> (Settings, Vec<Warning>) {
    let mut warnings: Vec<Warning> = front
        .repeated
        .iter()
        .map(|repeated| Warning {
            line: NonZeroUsize::new(repeated.line),
            message: repeated.message(),
        })
        .collect();
    let mut own = Settings::default();
    for Pair { key, value } in &front.pairs {
        let message = match own.set(key, value) {
            Ok(Given::First) => continue,
            Ok(Given::Again) => format!(
                "front matter: `{key}` sets what another name of it set already; `{key}` is used"
            ),
            Err(message) => format!("front matter: {message}"),
        };
        warnings.push(Warning {
            line: Some(NonZeroUsize::MIN),
            message,
        });
    }
    let folder = Folder::of(source);
    let mut refused = Vec::new();
    own.css = folder.admitted(&own.css, &["css"], &mut refused);
    own.head = folder.admitted(&own.head, &["html", "htm"], &mut refused);
    own.translated_from = own.translated_from.take().filter(|name| {
        folder
            .admits(Path::new(name), &MARKDOWN, &mut refused)
            .is_some()
    });
    warnings.extend(refused.into_iter().map(|message| Warning {
        line: Some(NonZeroUsize::MIN),
        message: format!("front matter: {message}; it is not read"),
    }));
    (own, warnings)
}

/// The document's folder, which the files it names must lie in.
struct Folder<'doc> {
    /// The folder, `.` for a bare file name; `None` for standard input.
    dir: Option<&'doc Path>,
}

impl<'doc> Folder<'doc> {
    /// The folder of `source`.
    fn of(source: Option<&'doc Path>) -> Self {
        let dir = source.and_then(Path::parent).map(|dir| {
            if dir.as_os_str().is_empty() {
                Path::new(".")
            } else {
                dir
            }
        });
        Self { dir }
    }

    /// Where `file` really is, when it is of one of `kinds` and inside the
    /// folder; why not goes to `refused`.
    fn admits(&self, file: &Path, kinds: &[&str], refused: &mut Vec<String>) -> Option<PathBuf> {
        match self.check(file, kinds) {
            Ok(real) => Some(real),
            Err(message) => {
                refused.push(message);
                None
            }
        }
    }

    /// The files of `files` that are of one of `kinds` and inside the
    /// folder, each where it really is and once, in their order; why each
    /// other is not goes to `refused`.
    ///
    /// The page reads each file where it really is, so a link changed
    /// after the check leads nowhere new; and a file named many times is
    /// read once, so naming one stylesheet thousands of times does not
    /// make a page thousands of times its size.
    fn admitted(
        &self,
        files: &[PathBuf],
        kinds: &[&str],
        refused: &mut Vec<String>,
    ) -> Vec<PathBuf> {
        let mut seen = BTreeSet::new();
        files
            .iter()
            .filter_map(|file| self.admits(file, kinds, refused))
            .filter(|real| seen.insert(real.clone()))
            .collect()
    }

    /// Why `file` may not be read, if it may not.
    ///
    /// # Errors
    ///
    /// `<file> is not a .css or .html file` (the kinds asked for) when its
    /// extension is not one of `kinds`, and `<file> is outside the
    /// document's folder` when it is an absolute path, does not exist, or
    /// resolves (links followed) to a place outside both the document's
    /// folder and the folder md2html runs in.
    fn check(&self, file: &Path, kinds: &[&str]) -> Result<PathBuf, String> {
        let known = file
            .extension()
            .is_some_and(|ext| kinds.iter().any(|kind| ext.eq_ignore_ascii_case(kind)));
        if !known {
            return Err(format!(
                "{} is not a .{} file",
                file.display(),
                kinds.join(" or .")
            ));
        }
        let full = self.dir.unwrap_or_else(|| Path::new(".")).join(file);
        let dirs: Vec<&Path> = self.dir.into_iter().collect();
        text::inside(&full, &dirs)
            .filter(|_| !file.is_absolute())
            .ok_or_else(|| format!("{} is outside the document's folder", file.display()))
    }
}

/// A file named relative to the Markdown's folder, read up to
/// [`MAX_BESIDE`].
///
/// # Errors
///
/// When the file is not a file (a folder, a pipe, a device), cannot be
/// opened or read, is not UTF-8, or is larger than
/// [`MAX_BESIDE`]; the message starts with the path that was tried.
pub(super) fn read_beside(base: Option<&Path>, file: &Path) -> Result<String, String> {
    let path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        base.unwrap_or_else(|| Path::new(".")).join(file)
    };
    read_named(&path, MAX_BESIDE)
        .map_err(|unread| format!("{}: {}", path.display(), unread.describe("")))
}
