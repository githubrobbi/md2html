// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The files of a run: which inputs a folder holds, which outputs md2html
//! refuses to write, and writing a file whole or not at all.

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write as _};
use std::path::{Component, Path, PathBuf};

use crate::document::MARKDOWN;
use crate::system;
use crate::text::PathUrl;

/// Whether a path names a Markdown file.
pub(super) fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            MARKDOWN
                .iter()
                .any(|markdown| ext.eq_ignore_ascii_case(markdown))
        })
}

/// Every Markdown file in folder `root`, as its path under `root`:
/// folders sorted, an index or README first. Hidden files and folders,
/// `target` and `node_modules` are left out.
///
/// # Errors
///
/// `<folder> cannot be read: <reason>` for the first folder under `root`,
/// `root` included, that cannot be listed: the Markdown in it would be
/// missing from the run without a word.
pub(super) fn walk(root: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    walk_from(root, Path::new(""), found)
}

/// [`walk`] in folder `dir`, which is `under` in the folder walked.
///
/// # Errors
///
/// As [`walk`].
fn walk_from(dir: &Path, under: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let unread = |error: std::io::Error| format!("{} cannot be read: {error}", dir.display());
    let mut entries: Vec<(OsString, PathBuf)> = Vec::new();
    for listed in std::fs::read_dir(dir).map_err(unread)? {
        let entry = listed.map_err(unread)?;
        entries.push((entry.file_name(), entry.path()));
    }
    entries.sort_by_key(|(name, path)| (path.is_dir(), !opens_folder(name), sort_name(name)));
    for (name, path) in entries {
        if name.as_encoded_bytes().starts_with(b".") || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk_from(&path, &under.join(&name), found)?;
        } else if is_markdown(&path) {
            found.push(under.join(&name));
        }
    }
    Ok(())
}

/// Whether a file's name makes it the first of its folder: `index.` or
/// `readme.` in any case.
fn opens_folder(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    [b"index.".as_slice(), b"readme."].iter().any(|start| {
        bytes
            .get(..start.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(start))
    })
}

/// The name a folder's entries are sorted by.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SortName {
    /// A Unicode name, lowercased.
    Text(String),
    /// A name that is not Unicode, by its bytes: after every Unicode name.
    Bytes(Vec<u8>),
}

/// The name `name` is sorted by.
fn sort_name(name: &OsStr) -> SortName {
    name.to_str().map_or_else(
        || SortName::Bytes(name.as_encoded_bytes().to_vec()),
        |text| SortName::Text(text.to_lowercase()),
    )
}

/// Why `out` must not be written: it is the source file itself, or
/// Markdown, wherever the Markdown comes from (`source` is `None` for
/// standard input).
pub(super) fn unsafe_output(source: Option<&Path>, out: &Path) -> Option<&'static str> {
    if source.is_some_and(|file| same_file(file, out)) {
        return Some("it is the input file");
    }
    is_markdown(out).then_some("the output would be a Markdown file")
}

/// Whether `source` and `out` name one file, links and case folding
/// followed where the file system has them; as written when either is not
/// there.
fn same_file(source: &Path, out: &Path) -> bool {
    match (std::fs::canonicalize(source), std::fs::canonicalize(out)) {
        (Ok(source_file), Ok(out_file)) => source_file == out_file,
        _ => source == out,
    }
}

/// Writes a file whole or not at all: a reader never sees half a page.
///
/// The bytes go to a temporary file beside it first, created new: a
/// symlink planted under the temporary name is not followed, so nothing
/// else is written through it. A temporary file a stopped run left behind
/// is removed first.
///
/// # Errors
///
/// `<path> names no file` for a path without a file name (`/`, `..`);
/// otherwise what failed, and the temporary file if it could not be
/// removed.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(name) = path.file_name() else {
        return Err(format!("{} names no file", path.display()));
    };
    let mut hidden = OsString::from(".");
    hidden.push(name);
    hidden.push(".md2html-tmp");
    let temporary = path.with_file_name(hidden);
    let mut file = match create_new(&temporary) {
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            std::fs::remove_file(&temporary)
                .map_err(|stale| format!("{} is in the way: {stale}", temporary.display()))?;
            create_new(&temporary)
        }
        created => created,
    }
    .map_err(|error| error.to_string())?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.flush())
        .and_then(|()| std::fs::rename(&temporary, path));
    let Err(error) = written else {
        return Ok(());
    };
    match std::fs::remove_file(&temporary) {
        Ok(()) => Err(error.to_string()),
        Err(left) if left.kind() == ErrorKind::NotFound => Err(error.to_string()),
        Err(left) => Err(format!(
            "{error}; {} is left behind ({left})",
            temporary.display()
        )),
    }
}

/// Creates `path`, which must not exist yet: not even as a symlink.
///
/// # Errors
///
/// The error of the file system: `AlreadyExists` when anything, a dangling
/// symlink included, is at `path`; otherwise why the file could not be
/// created there (no such folder, no permission).
fn create_new(path: &Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// The URL from page `from` to page `to`, for a link between them; none
/// when `to` names no file, or a name on the way is not Unicode, as a link
/// names a file by text.
///
/// Each name is one segment of the URL ([`system::relative_url`]): the
/// system's separator becomes `/`, and what a name holds that a URL would
/// read otherwise (a space, `#`, `?`, `%`, on Linux and macOS a `\`) is
/// escaped. Pages on two Windows drives share no folder to go up to: the
/// link is then the page's own `file:` URL.
pub(super) fn relative(from: &Path, to: &Path) -> Option<PathUrl> {
    let from_dir = folder_of(from);
    let to_dir = folder_of(to);
    if from_dir.components().next() != to_dir.components().next() {
        let page = to_dir.join(to.file_name()?);
        return page.to_str().map(|_| system::file_url(&page));
    }
    let common = from_dir
        .components()
        .zip(to_dir.components())
        .take_while(|(here, there)| here == there)
        .count();
    let mut link = PathBuf::new();
    for _ in from_dir.components().skip(common) {
        link.push(Component::ParentDir);
    }
    for part in to_dir.components().skip(common) {
        link.push(part);
    }
    link.push(to.file_name()?);
    system::relative_url(&link)
}

/// The folder a page is in, from the root of the file system.
///
/// It is not resolved on disk: the pages of a run are written one after
/// the other, and a folder that does not exist yet must read the same as
/// one that does.
fn folder_of(page: &Path) -> PathBuf {
    let folder = page.parent().unwrap_or_else(|| Path::new("."));
    std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf())
}
