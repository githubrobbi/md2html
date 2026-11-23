// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Consistent `--version` output for every MD2HTML binary.
//!
//! Every MD2HTML executable prints the same shape: a short one-liner by default
//! and a multi-line build fingerprint with `--version --verbose` / `-v`.
//!
//! - [`version_short!`] → `"<name>[.exe] <semver> (<sha>)"`
//! - [`version_long!`] → the short line plus commit date, rustc, target,
//!   profile
//!
//! The macros expand **in the calling crate**, so `CARGO_PKG_VERSION` resolves
//! to that binary's version and `option_env!("MD2HTML_GIT_SHA")` (and the other
//! `MD2HTML_*` build vars) resolve to what its `build.rs` stamped via
//! `emit_build_env` (the `build` feature). On Windows the name carries the real
//! `.exe` suffix so `--version` matches the on-disk filename.

/// Executable-name suffix for the compiled target.
///
/// `.exe` on Windows, empty elsewhere - so the version macros print a name that
/// matches the actual on-disk filename.
#[must_use]
pub const fn exe_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

/// Short one-line version: `"<name>[.exe] <semver> (<sha>)"`.
///
/// Expands in the caller, so `CARGO_PKG_VERSION` and `MD2HTML_GIT_SHA` are that
/// binary's. `$name` is the bare binary stem (e.g. `"md2htmld"`); the `.exe`
/// suffix is appended automatically on Windows.
#[macro_export]
macro_rules! version_short {
    ($name:literal) => {
        format!(
            "{}{} {} ({})",
            $name,
            $crate::exe_suffix(),
            env!("CARGO_PKG_VERSION"),
            option_env!("MD2HTML_GIT_SHA").unwrap_or("unknown"),
        )
    };
}

/// Multi-line build fingerprint for bug reports (`--version --verbose` / `-v`).
///
/// The short line followed by `commit` (sha + date), `rustc`, `target`, and
/// `profile`, each read from the `MD2HTML_*` env this crate's
/// `emit_build_env` (the `build` feature) stamps at build time (`"unknown"`
/// when a field could not be resolved).
#[macro_export]
macro_rules! version_long {
    ($name:literal) => {
        format!(
            "{}{} {}\n\
             commit:   {} ({})\n\
             rustc:    {}\n\
             target:   {}\n\
             profile:  {}",
            $name,
            $crate::exe_suffix(),
            env!("CARGO_PKG_VERSION"),
            option_env!("MD2HTML_GIT_SHA").unwrap_or("unknown"),
            option_env!("MD2HTML_COMMIT_DATE").unwrap_or("unknown"),
            option_env!("MD2HTML_RUSTC").unwrap_or("unknown"),
            option_env!("MD2HTML_TARGET").unwrap_or("unknown"),
            option_env!("MD2HTML_PROFILE").unwrap_or("unknown"),
        )
    };
}

/// The version text the command line asks for, or `None` when it asks for
/// something else.
///
/// Expands in the caller to [`version_request`] over the process arguments,
/// with the caller's own [`version_short!`] and [`version_long!`], so the
/// text carries that binary's `CARGO_PKG_VERSION` and `MD2HTML_GIT_SHA`.
/// `$name` is the bare binary stem (`"md2html-gen-hooks"`). The binary
/// writes the text and returns from `main`:
///
/// ```
/// use std::io::Write as _;
///
/// fn main() -> std::io::Result<()> {
///     if let Some(text) = md2html_version::requested_version!("md2html-gen-hooks") {
///         return writeln!(std::io::stdout().lock(), "{text}");
///     }
///     // ... the tool's own work ...
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! requested_version {
    ($name:literal) => {
        $crate::version_request(
            ::std::env::args_os(),
            || $crate::version_short!($name),
            || $crate::version_long!($name),
        )
    };
}

/// The version text `args` (the whole command line, program name first)
/// asks for: `short` for `--version` / `-V` as the first argument, `long`
/// when `--verbose` / `-v` follows it, `None` otherwise.
///
/// Only the first argument counts, so a later `--version` stays an argument
/// of the tool itself. Only the requested form is built.
///
/// ```
/// let text = md2html_version::version_request(
///     ["tool", "--version", "-v"].map(std::ffi::OsString::from),
///     || "short".to_owned(),
///     || "long".to_owned(),
/// );
/// assert_eq!(text.as_deref(), Some("long"));
/// ```
pub fn version_request<Args, Short, Long>(args: Args, short: Short, long: Long) -> Option<String>
where
    Args: IntoIterator<Item = std::ffi::OsString>,
    Short: FnOnce() -> String,
    Long: FnOnce() -> String,
{
    let mut rest = args.into_iter().skip(1);
    let first = rest.next()?;
    if first != "--version" && first != "-V" {
        return None;
    }
    if rest.any(|arg| arg == "--verbose" || arg == "-v") {
        Some(long())
    } else {
        Some(short())
    }
}

/// Tells cargo, through `out`, the build metadata the version macros read
/// (`cargo:rustc-env` lines).
///
/// Call once from a binary crate's `build.rs` (with the `build` feature on),
/// with the build script's standard output. Emits `MD2HTML_GIT_SHA` (short
/// HEAD sha, `-dirty` when the tree is modified), `MD2HTML_COMMIT_DATE`,
/// `MD2HTML_RUSTC`, `MD2HTML_TARGET`, and `MD2HTML_PROFILE`. A field that
/// cannot be resolved is stamped `unknown`, so a build with no git available
/// still succeeds.
///
/// It also names the files whose change re-stamps the build: the
/// repository's `HEAD` and the branch file it points to, wherever git keeps
/// them for the calling crate (a crate at any depth, a worktree whose `.git`
/// is a file, a packed branch that has no file of its own).
///
/// # Errors
///
/// `out` cannot be written.
#[cfg(feature = "build")]
pub fn emit_build_env<Out: std::io::Write>(out: &mut Out) -> std::io::Result<()> {
    writeln!(out, "cargo:rustc-env=MD2HTML_GIT_SHA={}", git_sha())?;
    writeln!(
        out,
        "cargo:rustc-env=MD2HTML_COMMIT_DATE={}",
        git_output(&["show", "-s", "--format=%cs", "HEAD"])
    )?;
    writeln!(out, "cargo:rustc-env=MD2HTML_RUSTC={}", rustc_version())?;
    writeln!(
        out,
        "cargo:rustc-env=MD2HTML_TARGET={}",
        env_or_unknown("TARGET")
    )?;
    writeln!(
        out,
        "cargo:rustc-env=MD2HTML_PROFILE={}",
        env_or_unknown("PROFILE")
    )?;
    // Re-stamp when the checked-out commit or the compiler changes.
    for path in head_files() {
        writeln!(out, "cargo:rerun-if-changed={path}")?;
    }
    writeln!(out, "cargo:rerun-if-env-changed=RUSTC")
}

/// The files that change when the checked-out commit does.
///
/// They are paths from the build script's directory (the calling crate's):
/// `HEAD`, and the branch file it names when that file exists (a packed
/// branch has none, and a missing file would make cargo re-run the script on
/// every build).  Empty without git.
#[cfg(feature = "build")]
fn head_files() -> Vec<String> {
    let Some(head) = git_line(&["rev-parse", "--git-path", "HEAD"]) else {
        return Vec::new();
    };
    let branch = git_line(&["symbolic-ref", "-q", "HEAD"])
        .and_then(|reference| git_line(&["rev-parse", "--git-path", &reference]))
        .filter(|path| std::path::Path::new(path).is_file());
    core::iter::once(head).chain(branch).collect()
}

/// Run `git <args>`, returning trimmed stdout, or `None` when git is
/// missing, fails or prints nothing.
#[cfg(feature = "build")]
fn git_line(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
}

/// Run `git <args>`, returning trimmed stdout or `"unknown"`.
#[cfg(feature = "build")]
fn git_output(args: &[&str]) -> String {
    git_line(args).unwrap_or_else(|| "unknown".to_owned())
}

/// Short HEAD sha with a `-dirty` suffix when the working tree has uncommitted
/// changes (so a hand-tweaked local build is never mistaken for the commit).
#[cfg(feature = "build")]
fn git_sha() -> String {
    let sha = git_output(&["rev-parse", "--short", "HEAD"]);
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_some_and(|out| !out.stdout.is_empty());
    if dirty && sha != "unknown" {
        format!("{sha}-dirty")
    } else {
        sha
    }
}

/// The building compiler's `rustc --version` line (via the `RUSTC` cargo sets).
#[cfg(feature = "build")]
fn rustc_version() -> String {
    // The compiler cargo names, whatever bytes its path holds.
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// A cargo-provided build env var (e.g. `TARGET`, `PROFILE`) or `"unknown"`.
#[cfg(feature = "build")]
fn env_or_unknown(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{exe_suffix, version_request};

    /// The text `version_request` gives for `line`, with `short` and `long`
    /// standing in for the two forms.
    fn asked(line: &[&str]) -> Option<String> {
        version_request(
            line.iter().map(OsString::from),
            || "short".to_owned(),
            || "long".to_owned(),
        )
    }

    /// `--version` and `-V` as the first argument ask for the short line.
    ///
    /// # Panics
    ///
    /// When either flag gives the long text or no text.
    #[test]
    fn version_flag_first_asks_for_the_short_line() {
        assert_eq!(asked(&["tool", "--version"]).as_deref(), Some("short"));
        assert_eq!(asked(&["tool", "-V"]).as_deref(), Some("short"));
    }

    /// A `--verbose` or `-v` after the version flag asks for the fingerprint.
    ///
    /// # Panics
    ///
    /// When `--version --verbose`, or `-V` with `-v` two arguments later, does
    /// not give the long text.
    #[test]
    fn verbose_after_the_version_flag_asks_for_the_fingerprint() {
        assert_eq!(
            asked(&["tool", "--version", "--verbose"]).as_deref(),
            Some("long")
        );
        assert_eq!(asked(&["tool", "-V", "x", "-v"]).as_deref(), Some("long"));
    }

    /// A line without the version flag first asks for no version.
    ///
    /// # Panics
    ///
    /// When the bare program name, a `--version` after a subcommand, or a `-v`
    /// before `--version` is taken for a version request.
    #[test]
    fn anything_else_is_not_a_version_request() {
        assert_eq!(asked(&["tool"]), None);
        assert_eq!(asked(&["tool", "check", "--version"]), None);
        assert_eq!(asked(&["tool", "-v", "--version"]), None);
    }

    /// The executable suffix is the target's.
    ///
    /// # Panics
    ///
    /// When `exe_suffix` is not `.exe` on Windows, or not empty elsewhere.
    #[test]
    fn exe_suffix_matches_target() {
        if cfg!(windows) {
            assert_eq!(exe_suffix(), ".exe");
        } else {
            assert_eq!(exe_suffix(), "");
        }
    }

    /// The short line is the name, the version and the commit in parentheses.
    ///
    /// # Panics
    ///
    /// When the line does not start with the name (with `.exe` on Windows),
    /// lacks this crate's `CARGO_PKG_VERSION`, or does not end in a
    /// parenthesised commit.
    #[test]
    fn version_short_has_name_semver_and_sha_shape() {
        // Expands in this (test) crate: CARGO_PKG_VERSION is md2html-version's.
        let line = version_short!("md2html-version");
        assert!(line.starts_with("md2html-version"));
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
        assert!(line.contains('(') && line.ends_with(')'));
        #[cfg(windows)]
        assert!(line.starts_with("md2html-version.exe "));
    }

    /// The fingerprint is the short line and four fields, one per line.
    ///
    /// # Panics
    ///
    /// When it has fewer than five lines, or lacks one of the `commit:`,
    /// `rustc:`, `target:` and `profile:` fields.
    #[test]
    fn version_long_is_multiline_with_fields() {
        let text = version_long!("md2html-version");
        assert!(text.lines().count() >= 5, "short line + 4 metadata lines");
        assert!(text.contains("commit:"));
        assert!(text.contains("rustc:"));
        assert!(text.contains("target:"));
        assert!(text.contains("profile:"));
    }
}
