// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What differs from one operating system to another, in one place.
//!
//! md2html's code has no `cfg` for a system. Where Windows, macOS and Linux
//! differ, the standard library says how (`Path::components` reads a drive
//! or a share on Windows only, `std::path::is_separator` is `\` as well as
//! `/` on Windows only, `EXE_SUFFIX` is `.exe` there), and this module holds
//! the rest: where the home folder is, how a path becomes a `file:` URL, and
//! which relative paths stay below the folder they start from.
//! `docs/policies/platform_policy.md` names every difference, where it is
//! handled and which test holds it on which system.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf, Prefix};

use crate::text::PathUrl;

/// The person's home folder: `HOME`, else `USERPROFILE`, which Windows
/// sets instead of `HOME`.
pub(crate) fn home() -> Option<PathBuf> {
    home_of(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// The home folder from the values of `HOME` and `USERPROFILE`: the first
/// that is set and not empty.
fn home_of(home: Option<OsString>, profile: Option<OsString>) -> Option<PathBuf> {
    home.into_iter()
        .chain(profile)
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// An absolute path as a `file:` URL.
///
/// The path is read part by part, the way the system reads it. On Windows
/// a drive (`C:`) or a share (`\\server\share`) leads, the verbatim forms
/// that `std::fs::canonicalize` gives (`\\?\C:\`, `\\?\UNC\server\share\`)
/// read as the plain ones, and `\` separates as `/` does; elsewhere `\` is
/// a character of a name. Each name is one segment of the URL's path
/// ([`PathUrl`]), so a name that is not UTF-8 keeps its own bytes
/// (`%E9`) and the URL names the file on disk.
pub(crate) fn file_url(absolute: &Path) -> PathUrl {
    let mut url = PathUrl::file();
    let mut named = false;
    for part in absolute.components() {
        match part {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => url.drive(drive),
                // The server is the URL's host: `file://server/share/…`.
                Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                    url.host(server.as_encoded_bytes());
                    url.push(share.as_encoded_bytes());
                }
                // A device or another verbatim name has no `file:` URL of
                // its own; its name stands as the first part.
                Prefix::Verbatim(name) | Prefix::DeviceNS(name) => {
                    url.push(name.as_encoded_bytes());
                }
            },
            Component::RootDir => continue,
            Component::CurDir | Component::ParentDir | Component::Normal(_) => {
                url.push(part.as_os_str().as_encoded_bytes());
            }
        }
        named = true;
    }
    // The root alone is `file:///`.
    if !named {
        url.push(b"");
    }
    url
}

/// A relative path as a relative URL; none when the path is not relative
/// or not Unicode.
///
/// Its parts, read with the system's separators (on Windows `\` as well
/// as `/`), are joined by `/`, each one segment ([`PathUrl::relative`]). A
/// root, a drive or a share leaves no relative URL, and a part that is not
/// Unicode none either, as a link names a file by text.
pub(crate) fn relative_url(path: &Path) -> Option<PathUrl> {
    let names = path
        .components()
        .map(|part| match part {
            Component::CurDir | Component::ParentDir | Component::Normal(_) => {
                part.as_os_str().to_str().map(str::as_bytes)
            }
            Component::Prefix(_) | Component::RootDir => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(PathUrl::relative(&names))
}

/// Why a relative path names no file below the folder it starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoFileBelow {
    /// It leaves the folder: a root, a drive or share, a `..`.
    Leaves,
    /// It names a folder: it is empty, or ends in a separator.
    Folder,
    /// It has an empty part (`a//b`) or a `.` after its start (`a/./b`):
    /// the system reads past them to a file, but the text and the file's
    /// names no longer agree.
    Loose,
}

/// The parts of a relative path that names a file below the folder it
/// starts from: names, and a `.` only at its start (`./pic.png`).
///
/// The parts are the system's: on Windows `\` separates as `/` does, and
/// `C:x` (a drive's current folder) or `\x` (the drive's root) leaves the
/// folder; elsewhere `\` is a character of a name. The rule is md2html's,
/// not the file system's: Windows opens `pic.png\` as the file `pic.png`
/// and Linux refuses it, but here a path ending in a separator names a
/// folder on every system.
///
/// # Errors
///
/// [`NoFileBelow`] says why the path names no such file.
pub(crate) fn file_below(path: &Path) -> Result<Vec<&OsStr>, NoFileBelow> {
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(_) | Component::CurDir => parts.push(part.as_os_str()),
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(NoFileBelow::Leaves);
            }
        }
    }
    if parts.is_empty() || ends_in_separator(path) {
        return Err(NoFileBelow::Folder);
    }
    // `components` passes over an empty part and a `.` after the start;
    // the text, split where the system separates, still has them.
    let written = path
        .as_os_str()
        .as_encoded_bytes()
        .split(|&byte| std::path::is_separator(char::from(byte)))
        .count();
    if written == parts.len() {
        Ok(parts)
    } else {
        Err(NoFileBelow::Loose)
    }
}

/// Whether `path` ends in one of the system's separators (`/`, and on
/// Windows `\` as well): it then names a folder.
pub(crate) fn ends_in_separator(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .last()
        .is_some_and(|&last| std::path::is_separator(char::from(last)))
}

#[cfg(test)]
mod tests;
