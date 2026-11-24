// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Tests of what differs by operating system. Those marked `cfg(windows)`
//! or `cfg(unix)` hold the rule of that system and run there: the CI tests
//! md2html on Linux, macOS and Windows.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::{NoFileBelow, file_below, file_url, home_of};
#[cfg(unix)]
use crate::tests::not_unicode;

/// The home folder is `HOME`, else `USERPROFILE`; an empty value is not
/// a folder.
///
/// # Panics
///
/// When `HOME` does not come first, `USERPROFILE` is not taken without
/// `HOME`, an empty value is taken, or a folder is found in neither.
#[test]
fn the_home_folder_is_home_else_the_user_profile() {
    let set = |value: &str| Some(OsString::from(value));
    assert_eq!(
        home_of(set("/home/a"), set(r"C:\Users\a")),
        Some(PathBuf::from("/home/a"))
    );
    assert_eq!(
        home_of(None, set(r"C:\Users\a")),
        Some(PathBuf::from(r"C:\Users\a"))
    );
    assert_eq!(
        home_of(set(""), set(r"C:\Users\a")),
        Some(PathBuf::from(r"C:\Users\a"))
    );
    assert_eq!(home_of(set(""), None), None);
    assert_eq!(home_of(None, None), None);
}

/// A path's parts become the URL's, each byte that is not plain escaped.
///
/// # Panics
///
/// When `/tmp/a b/c#d.html` is not `file:///tmp/a%20b/c%23d.html`, or the
/// root is not `file:///`.
#[test]
fn a_file_url_escapes_what_is_not_plain() {
    assert_eq!(
        file_url(Path::new("/tmp/a b/c#d.html")),
        "file:///tmp/a%20b/c%23d.html"
    );
    assert_eq!(file_url(Path::new("/")), "file:///");
}

/// A canonical path on this system gives a `file:` URL with an empty host
/// and the path after it (on Windows, the drive first).
///
/// # Panics
///
/// When the URL of the crate's own folder, resolved, does not start with
/// `file:///` and then `/` or a drive letter and a colon.
#[test]
fn a_resolved_folder_gives_a_plain_file_url() {
    let here = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let url = file_url(&here);
    let path = url.strip_prefix("file:///").unwrap();
    let drive = path.chars().take(2).collect::<Vec<_>>();
    let plain =
        !cfg!(windows) || matches!(drive.as_slice(), [letter, ':'] if letter.is_ascii_alphabetic());
    assert!(
        plain && !path.contains('?') && !path.contains('\\'),
        "{url}"
    );
}

/// On Linux and macOS a backslash is a character of a name, and a name
/// that is not UTF-8 keeps its bytes.
///
/// # Panics
///
/// When `C:\notes\a.html` is not one name, escaped, or `/tmp/caf\xe9 x.html`
/// is not `file:///tmp/caf%E9%20x.html`.
#[cfg(unix)]
#[test]
fn on_unix_a_backslash_is_part_of_a_name() {
    assert_eq!(
        file_url(Path::new("/srv/C:\\notes\\a.html")),
        "file:///srv/C:%5Cnotes%5Ca.html"
    );
    let odd = not_unicode("/tmp/caf", " x.html");
    assert_eq!(file_url(Path::new(&odd)), "file:///tmp/caf%E9%20x.html");
}

/// On Windows the drive leads, the verbatim forms read as the plain ones,
/// a share is the URL's host, and a backslash separates.
///
/// # Panics
///
/// When a drive path, its verbatim form, a share or the verbatim form of a
/// share does not give its `file:` URL.
#[cfg(windows)]
#[test]
fn on_windows_drives_and_shares_lead_the_url() {
    assert_eq!(
        file_url(Path::new(r"C:\notes\a b.html")),
        "file:///C:/notes/a%20b.html"
    );
    assert_eq!(
        file_url(Path::new(r"\\?\C:\notes\a.html")),
        "file:///C:/notes/a.html"
    );
    assert_eq!(
        file_url(Path::new(r"\\server\share\notes\a.html")),
        "file://server/share/notes/a.html"
    );
    assert_eq!(
        file_url(Path::new(r"\\?\UNC\server\share\a.html")),
        "file://server/share/a.html"
    );
    assert_eq!(
        file_url(Path::new("C:/mixed\\seps.html")),
        "file:///C:/mixed/seps.html"
    );
}

/// A relative path made of names gives its parts; one that leaves its
/// folder, names a folder, or is loose gives why not, on every system.
///
/// # Panics
///
/// When `./a/b.png` is not `.`, `a`, `b.png`; when `/a.png`, `../a.png` or
/// `a/../../b.png` does not leave; when the empty path, `a/` or `a.png//`
/// does not name a folder; or when `a//b.png`, `a/./b.png` or `a.png/.` is
/// not loose.
#[test]
fn only_names_stay_below() {
    assert_eq!(
        file_below(Path::new("./a/b.png")),
        Ok(vec![OsStr::new("."), OsStr::new("a"), OsStr::new("b.png")])
    );
    for (path, why) in [
        ("/a.png", NoFileBelow::Leaves),
        ("../a.png", NoFileBelow::Leaves),
        ("a/../../b.png", NoFileBelow::Leaves),
        ("", NoFileBelow::Folder),
        ("a/", NoFileBelow::Folder),
        ("a.png//", NoFileBelow::Folder),
        ("a//b.png", NoFileBelow::Loose),
        ("a/./b.png", NoFileBelow::Loose),
        ("a.png/.", NoFileBelow::Loose),
    ] {
        assert_eq!(file_below(Path::new(path)), Err(why), "{path:?}");
    }
}

/// On Linux and macOS a backslash is a character of a name, even the last.
///
/// # Panics
///
/// When `a\..\b.png` or `a.png\` is not one name.
#[cfg(unix)]
#[test]
fn on_unix_a_backslash_does_not_separate_names() {
    for one in [r"a\..\b.png", r"a.png\"] {
        assert_eq!(file_below(Path::new(one)), Ok(vec![OsStr::new(one)]));
    }
}

/// On Windows a backslash separates names, so one at the end names a
/// folder; and a drive, a drive's root or a share leaves the folder.
///
/// # Panics
///
/// When `a\b.png` is not `a`, `b.png`; when `a.png\` does not name a
/// folder or `a\\b.png` is not loose; or when `\x.png`, `C:x.png`,
/// `C:\x.png`, `\\server\share\x.png` or `a\..\..\x.png` does not leave.
#[cfg(windows)]
#[test]
fn on_windows_drives_and_roots_leave_the_folder() {
    assert_eq!(
        file_below(Path::new(r"a\b.png")),
        Ok(vec![OsStr::new("a"), OsStr::new("b.png")])
    );
    assert_eq!(file_below(Path::new(r"a.png\")), Err(NoFileBelow::Folder));
    assert_eq!(file_below(Path::new(r"a\\b.png")), Err(NoFileBelow::Loose));
    for leaving in [
        r"\x.png",
        "C:x.png",
        r"C:\x.png",
        r"\\server\share\x.png",
        r"a\..\..\x.png",
    ] {
        assert_eq!(
            file_below(Path::new(leaving)),
            Err(NoFileBelow::Leaves),
            "{leaving}"
        );
    }
}
