// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The files a document names beside it, and its own settings.
//!
//! Files the document itself names stay inside its folder (or the folder
//! md2html runs in), with the kind of name their job needs: an untrusted
//! Markdown file cannot pull a secret into its page. The command line may
//! name any file.

use alloc::collections::BTreeMap;
use std::path::Path;

use crate::config::Settings;
use crate::input::{MAX_BESIDE, read_file};
use crate::render::Warning;
use crate::text;

/// The extensions of a Markdown file.
pub(crate) const MARKDOWN: [&str; 4] = ["md", "markdown", "mdown", "mkd"];

/// The settings of the front matter, with the files that may not be read
/// taken out; what was wrong with them, as warnings.
pub(super) fn own_settings(
    map: &BTreeMap<String, String>,
    source: Option<&Path>,
) -> (Settings, Vec<Warning>) {
    let mut warnings = Vec::new();
    let mut own = Settings::default();
    for (key, value) in map {
        if let Err(message) = own.set(key, value) {
            warnings.push(Warning {
                line: 1,
                message: format!("front matter: {message}"),
            });
        }
    }
    let folder = Folder::of(source);
    let mut refused = Vec::new();
    own.css
        .retain(|file| folder.admits(file, &["css"], &mut refused));
    own.head
        .retain(|file| folder.admits(file, &["html", "htm"], &mut refused));
    own.mermaid_js = own
        .mermaid_js
        .take()
        .filter(|file| folder.admits(file, &["js", "mjs"], &mut refused));
    own.translated_from = own
        .translated_from
        .take()
        .filter(|name| folder.admits(Path::new(name), &MARKDOWN, &mut refused));
    warnings.extend(refused.into_iter().map(|message| Warning {
        line: 1,
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

    /// Whether `file` is of one of `kinds` and inside the folder; why not
    /// goes to `refused`.
    fn admits(&self, file: &Path, kinds: &[&str], refused: &mut Vec<String>) -> bool {
        match self.check(file, kinds) {
            Ok(()) => true,
            Err(message) => {
                refused.push(message);
                false
            }
        }
    }

    /// Why `file` may not be read, if it may not.
    fn check(&self, file: &Path, kinds: &[&str]) -> Result<(), String> {
        let ext = file
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !kinds.contains(&ext.as_str()) {
            return Err(format!(
                "{} is not a .{} file",
                file.display(),
                kinds.join(" or .")
            ));
        }
        let full = self.dir.unwrap_or_else(|| Path::new(".")).join(file);
        let dirs: Vec<&Path> = self.dir.into_iter().collect();
        if file.is_absolute() || !text::inside(&full, &dirs) {
            return Err(format!(
                "{} is outside the document's folder",
                file.display()
            ));
        }
        Ok(())
    }
}

/// A file named relative to the Markdown's folder, read up to
/// [`MAX_BESIDE`].
pub(super) fn read_beside(base: Option<&Path>, file: &Path) -> Result<String, String> {
    let path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        base.unwrap_or_else(|| Path::new(".")).join(file)
    };
    read_file(&path, MAX_BESIDE)
        .map_err(|unread| format!("{}: {}", path.display(), unread.describe("")))
}
