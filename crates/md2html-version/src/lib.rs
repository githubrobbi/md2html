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
//!
//! # Features
//!
//! - `build` (off by default) adds one function, `emit_build_env`, for a
//!   binary's `build.rs`; it takes nothing away and adds no dependency. The
//!   tooling turns it on only in `[build-dependencies]`, so no binary compiles
//!   it in. The crate is not published, so the feature is a contract within
//!   this workspace, not a semver promise.

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
/// suffix is appended automatically on Windows. A macro because `env!` and
/// `option_env!` read the crate they are written in: a function here would
/// give this crate's version, not the binary's.
///
/// ```
/// let line = md2html_version::version_short!("md2html-gen-hooks");
/// // `md2html-gen-hooks <version> (<commit>)`, `unknown` for a build without the stamp.
/// assert!(line.starts_with("md2html-gen-hooks"));
/// assert!(line.contains(env!("CARGO_PKG_VERSION")));
/// assert!(line.ends_with(')'));
/// ```
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
/// when a field could not be resolved). A macro for the reason
/// [`version_short!`] is one.
///
/// ```
/// let text = md2html_version::version_long!("md2html-gen-hooks");
/// let fields: Vec<&str> = text
///     .lines()
///     .skip(1)
///     .filter_map(|line| line.split(':').next())
///     .collect();
/// assert_eq!(fields, ["commit", "rustc", "target", "profile"]);
/// ```
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
/// with the build script's standard output. Emits `MD2HTML_GIT_SHA` (the
/// short sha of the checked-out commit), `MD2HTML_COMMIT_DATE`,
/// `MD2HTML_RUSTC`, `MD2HTML_TARGET`, and `MD2HTML_PROFILE`. A field that
/// cannot be resolved is stamped `unknown`, so a build with no git available
/// still succeeds.
///
/// Each stamp is a function of what this declares to cargo, so a build is
/// never stale and never re-stamped for nothing:
///
/// - The commit and its date come from the repository the calling crate is in:
///   git runs in the build script's directory with its own repository-locating
///   variables (`GIT_DIR`, `GIT_WORK_TREE` and the rest of `git rev-parse
///   --local-env-vars`) removed, because a git hook sets `GIT_DIR` and the
///   stamp must not depend on who started the build. They are read again when
///   the repository's `HEAD` or the branch it names moves, wherever git keeps
///   them (a crate at any depth, a worktree whose `.git` is a file); for a
///   packed branch, which has no file of its own, the folder git writes the
///   branch's file into at its next commit is watched instead.
/// - The compiler's version is read again when `RUSTC` changes.
/// - `TARGET` and `PROFILE` are cargo's own.
///
/// The working tree is not read: whether it holds uncommitted changes is
/// nothing cargo can be told to watch, so no stamp claims to know it.
///
/// A build script passes its locked standard output; any writer will do:
///
/// ```
/// let mut out = Vec::new();
/// md2html_version::emit_build_env(&mut out)?;
/// assert!(out.starts_with(b"cargo:rustc-env=MD2HTML_GIT_SHA="));
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Errors
///
/// `out` cannot be written.
#[cfg(feature = "build")]
pub fn emit_build_env<Out: std::io::Write>(out: &mut Out) -> std::io::Result<()> {
    let git = Git::in_dir(std::path::Path::new("."));
    writeln!(
        out,
        "cargo:rustc-env=MD2HTML_GIT_SHA={}",
        git.or_unknown(&["rev-parse", "--short", "HEAD"])
    )?;
    writeln!(
        out,
        "cargo:rustc-env=MD2HTML_COMMIT_DATE={}",
        git.or_unknown(&["show", "-s", "--format=%cs", "HEAD"])
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
    for path in git.head_files() {
        writeln!(out, "cargo:rerun-if-changed={path}")?;
    }
    writeln!(out, "cargo:rerun-if-env-changed=RUSTC")
}

/// Git, run on the repository one directory is in, whatever the environment
/// names.
#[cfg(feature = "build")]
struct Git {
    /// The directory git runs in (`git -C`).
    dir: std::path::PathBuf,
    /// Git's repository-local variables (`git rev-parse --local-env-vars`),
    /// removed from every run so the repository is the directory's.
    local_vars: Vec<String>,
}

#[cfg(feature = "build")]
impl Git {
    /// Git in `dir`; it asks git which variables locate a repository (none
    /// without git).
    fn in_dir(dir: &std::path::Path) -> Self {
        let local_vars = std::process::Command::new("git")
            .args(["rev-parse", "--local-env-vars"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map_or_else(Vec::new, |text| text.lines().map(str::to_owned).collect());
        Self {
            dir: dir.to_path_buf(),
            local_vars,
        }
    }

    /// `git -C <dir> <args>`, without the repository-local variables.
    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command.arg("-C").arg(&self.dir).args(args);
        for name in &self.local_vars {
            command.env_remove(name);
        }
        command
    }

    /// The trimmed standard output of `git <args>`, or `None` when git is
    /// missing, fails or prints nothing.
    fn line(&self, args: &[&str]) -> Option<String> {
        self.command(args)
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|raw| raw.trim().to_owned())
            .filter(|trimmed| !trimmed.is_empty())
    }

    /// The trimmed standard output of `git <args>`, or `"unknown"`.
    fn or_unknown(&self, args: &[&str]) -> String {
        self.line(args).unwrap_or_else(|| "unknown".to_owned())
    }

    /// What changes when the checked-out commit does, as paths from the
    /// directory: `HEAD`, and the branch it names. The branch is its file
    /// when it has one; a packed branch has none, and its next commit
    /// writes one into the folder of branch files, so that folder is
    /// watched (cargo watches a folder by everything in it). A path that
    /// does not exist is never named: cargo would run the script on every
    /// build. A detached `HEAD` holds the commit itself. Empty without git.
    fn head_files(&self) -> Vec<String> {
        let Some(head) = self.line(&["rev-parse", "--git-path", "HEAD"]) else {
            return Vec::new();
        };
        let branch = self
            .line(&["symbolic-ref", "-q", "HEAD"])
            .and_then(|reference| {
                self.line(&["rev-parse", "--git-path", &reference])
                    .filter(|file| self.dir.join(file).is_file())
                    .or_else(|| {
                        self.line(&["rev-parse", "--git-path", "refs/heads"])
                            .filter(|folder| self.dir.join(folder).is_dir())
                    })
            });
        core::iter::once(head).chain(branch).collect()
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

    /// `emit_build_env` stamps every field the macros read, names only files
    /// that exist for cargo to watch, and watches the compiler last.
    ///
    /// # Panics
    ///
    /// When a line is not a `cargo:` directive, one of the five `MD2HTML_*`
    /// variables is missing, a `rerun-if-changed` path does not exist (cargo
    /// would then run the build script on every build), or the last line is
    /// not the `RUSTC` watch.
    #[cfg(feature = "build")]
    #[test]
    fn emit_build_env_stamps_every_field_and_watches_existing_files() {
        let mut out = Vec::new();
        super::emit_build_env(&mut out).expect("a Vec takes every write");
        let text = String::from_utf8(out).expect("the directives are UTF-8");
        assert!(
            text.lines().all(|line| line.starts_with("cargo:")),
            "{text}"
        );
        for key in [
            "MD2HTML_GIT_SHA",
            "MD2HTML_COMMIT_DATE",
            "MD2HTML_RUSTC",
            "MD2HTML_TARGET",
            "MD2HTML_PROFILE",
        ] {
            let stamp = format!("cargo:rustc-env={key}=");
            assert!(text.lines().any(|line| line.starts_with(&stamp)), "{key}");
        }
        for path in text
            .lines()
            .filter_map(|line| line.strip_prefix("cargo:rerun-if-changed="))
        {
            assert!(std::path::Path::new(path).exists(), "{path}");
        }
        assert_eq!(
            text.lines().last(),
            Some("cargo:rerun-if-env-changed=RUSTC")
        );
    }

    /// Runs `git <args>` in `git`'s directory and gives its trimmed output.
    ///
    /// # Panics
    ///
    /// When git cannot be started, fails, or prints something that is not
    /// UTF-8.
    #[cfg(feature = "build")]
    fn run(git: &super::Git, args: &[&str]) -> String {
        let out = git.command(args).output().expect("git starts");
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout)
            .expect("git prints UTF-8")
            .trim()
            .to_owned()
    }

    /// A packed branch is watched through the folder its next commit writes
    /// its file into, and a branch with a file through that file, in a
    /// repository of its own whatever `GIT_DIR` the test runs under.
    ///
    /// # Panics
    ///
    /// When the temporary repository cannot be made or removed, the packed
    /// branch is not watched through its folder, or the loose branch not
    /// through its file.
    #[cfg(feature = "build")]
    #[test]
    fn a_packed_branch_is_watched_through_its_folder() {
        let dir =
            std::env::temp_dir().join(format!("md2html-version-head-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("make the repository's folder");
        let git = super::Git::in_dir(&dir);
        run(&git, &["-c", "init.defaultRefFormat=files", "init", "-q"]);
        run(&git, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        let tree = run(&git, &["write-tree"]);
        let identity = ["-c", "user.name=test", "-c", "user.email=test@example.com"];
        let first = run(
            &git,
            &[&identity[..], &[
                "commit-tree",
                "--no-gpg-sign",
                "-m",
                "one",
                &tree,
            ]]
            .concat(),
        );
        run(&git, &["update-ref", "refs/heads/main", &first]);
        run(&git, &["pack-refs", "--all"]);
        let branch_file = dir.join(run(&git, &["rev-parse", "--git-path", "refs/heads/main"]));
        assert!(!branch_file.exists(), "the branch is packed");

        let packed = git.head_files();
        assert_eq!(packed.len(), 2, "{packed:?}");
        assert!(
            packed.iter().all(|path| dir.join(path).exists()),
            "{packed:?}"
        );
        let folder = packed.get(1).expect("the packed branch is watched");
        assert!(dir.join(folder).is_dir(), "{packed:?}");
        assert!(folder.ends_with("refs/heads"), "{packed:?}");

        let second = run(
            &git,
            &[&identity[..], &[
                "commit-tree",
                "--no-gpg-sign",
                "-p",
                &first,
                "-m",
                "two",
                &tree,
            ]]
            .concat(),
        );
        run(&git, &["update-ref", "refs/heads/main", &second]);
        let loose = git.head_files();
        assert_eq!(loose.len(), 2, "{loose:?}");
        let file = loose.get(1).expect("the loose branch is watched");
        assert!(dir.join(file).is_file(), "{loose:?}");
        assert!(file.ends_with("refs/heads/main"), "{loose:?}");

        std::fs::remove_dir_all(&dir).expect("remove the repository");
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
