// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What differs from one operating system to another, in one place.
//!
//! md2html's code has no `cfg` for a system. Where Windows, macOS and Linux
//! differ, the standard library says how (`Path::components` reads a drive
//! or a share on Windows only, `std::path::is_separator` is `\` as well as
//! `/` on Windows only, `EXE_SUFFIX` is `.exe` there), and this module holds
//! the rest: where the home folder is, and how a path becomes a `file:`
//! URL.
//! `docs/policies/platform_policy.md` names every difference, where it is
//! handled and which test holds it on which system.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf, Prefix};

use crate::text::put;

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
/// a character of a name. Each byte that is not plain in a URL is escaped,
/// so a name that is not UTF-8 keeps its own bytes (`%E9`) and the URL
/// names the file on disk.
pub(crate) fn file_url(absolute: &Path) -> String {
    let mut out = String::from("file://");
    for part in absolute.components() {
        match part {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => {
                    out.push('/');
                    out.push(char::from(drive));
                    out.push(':');
                }
                // The server is the URL's host: `file://server/share/…`.
                Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                    escape(&mut out, server);
                    out.push('/');
                    escape(&mut out, share);
                }
                // A device or another verbatim name has no `file:` URL of
                // its own; its name stands as the first part.
                Prefix::Verbatim(name) | Prefix::DeviceNS(name) => {
                    out.push('/');
                    escape(&mut out, name);
                }
            },
            Component::RootDir => {}
            Component::CurDir | Component::ParentDir | Component::Normal(_) => {
                out.push('/');
                escape(&mut out, part.as_os_str());
            }
        }
    }
    if out.len() == "file://".len() {
        out.push('/');
    }
    out
}

/// Appends `name` to `out`, each byte that is not plain in a URL escaped.
fn escape(out: &mut String, name: &OsStr) {
    for &byte in name.as_encoded_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~:".contains(&byte) {
            out.push(char::from(byte));
        } else {
            put!(out, "%{byte:02X}");
        }
    }
}

#[cfg(test)]
mod tests;
