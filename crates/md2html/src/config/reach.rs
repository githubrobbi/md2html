// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The folders a document may make md2html read from: its own, and the one
//! the command line adds with `--root`.
//!
//! A document names files (stylesheets, a head file, a translation's
//! original, images to embed, to pack into a book or to print). Its author is
//! not always the person running md2html, so what it names is read only from
//! its own folder and below; the person running md2html may add one folder,
//! on the command line alone. No folder is allowed for being where md2html
//! runs: a document someone sent, rendered from the home folder, cannot reach
//! the rest of it.

use std::path::{Path, PathBuf};

use crate::text;

/// The folder `--root` names, where it really is: every document of the run
/// may read from it and below as from its own folder.
///
/// Only the command line gives one; the `root` setting of the front matter
/// is refused, so a document cannot widen what it may read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Root(PathBuf);

impl Root {
    /// The folder `dir` names, links resolved.
    ///
    /// # Errors
    ///
    /// `--root <dir>: <reason>` when it is not there or cannot be resolved,
    /// and `--root <dir>: not a folder` when it is a file (or anything else
    /// but a folder).
    pub(crate) fn folder(dir: &Path) -> Result<Self, String> {
        let real = std::fs::canonicalize(dir)
            .map_err(|error| format!("--root {}: {error}", dir.display()))?;
        if real.is_dir() {
            Ok(Self(real))
        } else {
            Err(format!("--root {}: not a folder", dir.display()))
        }
    }

    /// The folder, links resolved.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

/// The folder a handbook was found in, where it really is.
///
/// The command line named it; every chapter may read images from it and
/// below as from its own folder (stylesheets, heads and a translation's
/// original still only from its own).
///
/// The folder md2html runs in is not one for being that, as for `--root`:
/// `md2html .` gives each page its own folder only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BookFolder {
    /// The folder, links resolved.
    real: PathBuf,
    /// The folder as the command line names it, for messages.
    shown: String,
}

impl BookFolder {
    /// The folder `named` on the command line, links resolved; none when
    /// it cannot be resolved, or is the folder md2html runs in.
    pub(crate) fn named(named: &Path) -> Option<Self> {
        let real = std::fs::canonicalize(named).ok()?;
        let here = std::env::current_dir().and_then(std::fs::canonicalize).ok();
        if here.as_deref() == Some(real.as_path()) {
            return None;
        }
        Some(Self {
            real,
            shown: named.display().to_string(),
        })
    }

    /// The folder, links resolved.
    pub(crate) fn path(&self) -> &Path {
        &self.real
    }

    /// The folder as the command line names it.
    pub(crate) fn shown(&self) -> &str {
        &self.shown
    }
}

/// The folders one document may make md2html read from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reach<'dirs> {
    /// The document's own folders: none for standard input, which has
    /// none.
    pub(crate) own: &'dirs [&'dirs Path],
    /// The folder the command line adds, if it adds one.
    pub(crate) root: Option<&'dirs Root>,
}

impl Reach<'_> {
    /// Where `path` really is, links resolved, when that lies inside one of
    /// the folders ([`text::inside`]); none when it lies outside them all,
    /// or is not there.
    pub(crate) fn inside(&self, path: &Path) -> Option<PathBuf> {
        let dirs: Vec<&Path> = self
            .own
            .iter()
            .copied()
            .chain(self.root.map(Root::path))
            .collect();
        text::inside(path, &dirs)
    }

    /// Where a file is that lies in none of the folders, and how the person
    /// running md2html may allow it: what follows `is` after the file's
    /// name.
    pub(crate) fn outside(&self) -> String {
        match (self.own, self.root) {
            ([], None) => String::from(
                "outside any folder: standard input has none (--root <folder> allows one)",
            ),
            (_, None) => String::from(
                "outside the document's folder (--root <folder> allows a folder that holds it)",
            ),
            ([], Some(root)) => format!("outside --root {}", root.path().display()),
            (_, Some(root)) => format!(
                "outside the document's folder and --root {}",
                root.path().display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Reach, Root};

    /// A scratch folder of its own for one test, under the system's
    /// temporary folder.
    ///
    /// # Panics
    ///
    /// When it cannot be made.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("md2html-reach-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(dir.join("doc")).unwrap();
        dir
    }

    /// `--root` names a folder that is there; anything else is refused with
    /// the value as given.
    ///
    /// # Panics
    ///
    /// When a folder is refused or kept other than where it really is, or a
    /// file or a missing path is taken, or refused without its name.
    #[test]
    fn a_root_is_a_folder_that_is_there() {
        let dir = scratch("folder");
        std::fs::write(dir.join("file.txt"), "x").unwrap();
        let root = Root::folder(&dir.join("doc/..")).unwrap();
        assert_eq!(root.path(), std::fs::canonicalize(&dir).unwrap());
        let file = dir.join("file.txt");
        assert_eq!(
            Root::folder(&file),
            Err(format!("--root {}: not a folder", file.display()))
        );
        let missing = dir.join("missing");
        let refused = Root::folder(&missing).unwrap_err();
        assert!(
            refused.starts_with(&format!("--root {}: ", missing.display())),
            "{refused}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A document reads from its own folder and the root, and from nowhere
    /// else: not from the folder md2html runs in, and nothing at all from
    /// standard input without a root.
    ///
    /// # Panics
    ///
    /// When a file above the document's folder is read without a root or
    /// not read with one, when a file of the folder the tests run in is read
    /// without being in a folder given, or when the reason a file is not
    /// read does not name `--root`.
    #[test]
    fn a_document_reads_from_its_folder_and_the_root_only() {
        let dir = scratch("reach");
        std::fs::write(dir.join("doc/in.css"), "a").unwrap();
        std::fs::write(dir.join("up.css"), "b").unwrap();
        let doc = dir.join("doc");
        let own = [doc.as_path()];
        let alone = Reach {
            own: &own,
            root: None,
        };
        assert!(alone.inside(&doc.join("in.css")).is_some());
        assert_eq!(alone.inside(&doc.join("../up.css")), None);
        assert!(alone.outside().contains("--root"), "{}", alone.outside());
        // The tests run in the crate's folder, where `Cargo.toml` is.
        assert_eq!(alone.inside(Path::new("Cargo.toml")), None);
        let stdin = Reach {
            own: &[],
            root: None,
        };
        assert_eq!(stdin.inside(&doc.join("in.css")), None);
        assert_eq!(stdin.inside(Path::new("Cargo.toml")), None);
        assert!(stdin.outside().contains("--root"), "{}", stdin.outside());
        let root = Root::folder(&dir).unwrap();
        let widened = Reach {
            own: &own,
            root: Some(&root),
        };
        assert_eq!(
            widened.inside(&doc.join("../up.css")),
            Some(root.path().join("up.css"))
        );
        assert_eq!(widened.inside(Path::new("Cargo.toml")), None);
        let stdin_rooted = Reach {
            own: &[],
            root: Some(&root),
        };
        assert!(stdin_rooted.inside(&doc.join("in.css")).is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
