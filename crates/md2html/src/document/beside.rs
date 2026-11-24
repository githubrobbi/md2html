// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The files a document names beside it, and its own settings.
//!
//! Files the document itself names stay inside its folder (or the folder
//! `--root` adds), with the kind of name their job needs: an untrusted
//! Markdown file cannot pull a secret into its page. The command line may
//! name any file.

use alloc::collections::BTreeSet;
use core::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use crate::config::{FrontMatter, Given, Pair, Reach, Root, Settings};
use crate::input::{MARKDOWN, MAX_BESIDE, read_named};
use crate::render::Warning;

/// The settings of the front matter, the files that may not be read taken
/// out; what was wrong, as warnings.
///
/// A key given twice, a setting given by two of its names, a key md2html
/// does not know and a file it may not read are each told. The files are
/// read from the document's folder and from `root`, the folder the command
/// line adds; the front matter cannot add one.
pub(super) fn own_settings(
    front: &FrontMatter<'_>,
    source: Option<&Path>,
    root: Option<&Root>,
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
    let folder = Folder::of(source, root);
    let mut refused = Vec::new();
    own.css = folder.admitted(&own.css, &STYLESHEET, &mut refused);
    own.head = folder.admitted(&own.head, &HEAD, &mut refused);
    own.translated_from = own.translated_from.take().filter(|name| {
        folder
            .admits(Path::new(name), &MARKDOWN_FILE, &mut refused)
            .is_some()
    });
    warnings.extend(refused.into_iter().map(|message| Warning {
        line: Some(NonZeroUsize::MIN),
        message: format!("front matter: {message}; it is not read"),
    }));
    (own, warnings)
}

/// A kind of file a document may name, by its extensions.
struct Kind {
    /// Its extensions, in lower case.
    exts: &'static [&'static str],
    /// What the author is told it must be: `a {said} file`.
    said: &'static str,
}

/// A stylesheet.
const STYLESHEET: Kind = Kind {
    exts: &["css"],
    said: ".css",
};

/// HTML for the head.
const HEAD: Kind = Kind {
    exts: &["html", "htm"],
    said: ".html or .htm",
};

/// A Markdown file, by any of its extensions.
const MARKDOWN_FILE: Kind = Kind {
    exts: &MARKDOWN,
    said: "Markdown",
};

/// The document's folder, which the files it names must lie in, or the
/// folder the command line adds.
struct Folder<'doc> {
    /// The folder, `.` for a bare file name; `None` for standard input.
    dir: Option<&'doc Path>,
    /// The folder `--root` adds.
    root: Option<&'doc Root>,
}

impl<'doc> Folder<'doc> {
    /// The folder of `source`, and `root`.
    fn of(source: Option<&'doc Path>, root: Option<&'doc Root>) -> Self {
        let dir = source.and_then(Path::parent).map(|dir| {
            if dir.as_os_str().is_empty() {
                Path::new(".")
            } else {
                dir
            }
        });
        Self { dir, root }
    }

    /// Where `file` really is, when it is of the `kind` and inside the
    /// folder; why not goes to `refused`.
    fn admits(&self, file: &Path, kind: &Kind, refused: &mut Vec<String>) -> Option<PathBuf> {
        match self.check(file, kind) {
            Ok(real) => Some(real),
            Err(message) => {
                refused.push(message);
                None
            }
        }
    }

    /// The files of `files` that are of the `kind` and inside the
    /// folder, each where it really is and once, in their order; why each
    /// other is not goes to `refused`.
    ///
    /// The page reads each file where it really is, so a link changed
    /// after the check leads nowhere new; and a file named many times is
    /// read once, so naming one stylesheet thousands of times does not
    /// make a page thousands of times its size.
    fn admitted(&self, files: &[PathBuf], kind: &Kind, refused: &mut Vec<String>) -> Vec<PathBuf> {
        let mut seen = BTreeSet::new();
        files
            .iter()
            .filter_map(|file| self.admits(file, kind, refused))
            .filter(|real| seen.insert(real.clone()))
            .collect()
    }

    /// Why `file` may not be read, if it may not.
    ///
    /// # Errors
    ///
    /// `<file> is not a .css file` (the kind asked for) when its
    /// extension is not one of the `kind`'s, and `<file> is outside the
    /// document's folder` (with how `--root` allows it, [`Reach::outside`])
    /// when it is an absolute path, does not exist, or resolves (links
    /// followed) to a place outside both the document's folder and the
    /// root.
    fn check(&self, file: &Path, kind: &Kind) -> Result<PathBuf, String> {
        let known = file
            .extension()
            .is_some_and(|ext| kind.exts.iter().any(|one| ext.eq_ignore_ascii_case(one)));
        if !known {
            return Err(format!("{} is not a {} file", file.display(), kind.said));
        }
        let full = self.dir.unwrap_or_else(|| Path::new(".")).join(file);
        let reach = Reach {
            own: self.dir.as_slice(),
            root: self.root,
        };
        reach
            .inside(&full)
            .filter(|_| !file.is_absolute())
            .ok_or_else(|| format!("{} is {}", file.display(), reach.outside()))
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
