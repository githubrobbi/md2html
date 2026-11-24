// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! `just release-local`: the release archives for every platform this machine
//! can build, in `dist/`. Nothing is uploaded.
//!
//! Each platform is built optimised with the driver that serves it from here
//! (`cargo build`, `cargo zigbuild` or `cargo xwin build`), checked (the file
//! is the kind of executable the platform runs, and the one for this machine
//! answers with this version), and packed the way the release workflow packs
//! it: `md2html-<platform>.zip` holding one folder with the binary, the README
//! and the licences. `CHECKSUMS.txt`, and its copy `SHA256SUMS`, list the
//! SHA-256 of every archive.
//!
//! ```text
//! just release-local                      # every platform
//! just release-local linux-x64 windows-x64
//! just release-local --list
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, anyhow, bail};
use md2html_dev_tools::{
    Driver, PLATFORMS, Platform, VERSION, driver, executable_kind, fail, human_size, platform, run,
    say, sha256_hex, stdout_text, target_dir, workspace_root,
};

/// What goes into every archive beside the binary: the files
/// `release.yml`'s `stage_bundle` puts there, so an archive made here is
/// the one a release ships.
const BESIDE: [&str; 4] = ["README.md", "CHANGELOG.md", "LICENSE-MIT", "LICENSE-APACHE"];

/// What the command line asked for.
enum Asked {
    /// Build platforms.
    Build {
        /// The platforms to build.
        platforms: Vec<Platform>,
        /// Whether that is every platform: then `dist/` is cleared first.
        all: bool,
    },
    /// The list of platforms, and how each is built here.
    List,
    /// The usage.
    Help,
}

/// Reads the command line: platform names, `--list` or `--help`.
///
/// # Errors
///
/// An argument before any `--list`, `--help` or `-h` is not the name of a
/// platform md2html is built for; the message names it and points to
/// `--list`.
fn asked(args: &[String]) -> Result<Asked> {
    let mut platforms = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--list" => return Ok(Asked::List),
            "--help" | "-h" => return Ok(Asked::Help),
            name => platforms.push(platform(name).ok_or_else(|| {
                anyhow!("`{name}` is not a platform md2html is built for; see --list")
            })?),
        }
    }
    if platforms.is_empty() {
        Ok(Asked::Build {
            platforms: PLATFORMS.to_vec(),
            all: true,
        })
    } else {
        Ok(Asked::Build {
            platforms,
            all: false,
        })
    }
}

/// Lists every platform, its target and the driver that builds it here.
///
/// # Errors
///
/// Standard output cannot be written (from [`say`]).
fn list() -> Result<()> {
    for one in &PLATFORMS {
        let how = driver(std::env::consts::OS, one).map_or_else(
            || "not from this machine (only a Mac builds for macOS)".to_owned(),
            |with| format!("cargo {}", with.words().join(" ")),
        );
        say(format_args!("{:<14} {:<28} {how}", one.name, one.target))?;
    }
    Ok(())
}

/// Whether a platform is this machine's own, so its binary can be run here.
fn is_this_machine(one: &Platform) -> bool {
    one.os == std::env::consts::OS && one.arch == std::env::consts::ARCH
}

/// Builds one platform and returns its binary.
///
/// # Errors
///
/// `rustup target add` for the platform's target cannot be started or
/// fails, or the cargo build cannot be started or fails (from [`run`]); a
/// failed build's message says what the driver needs installed.
fn build(one: &Platform, with: Driver) -> Result<PathBuf> {
    // The target's standard library, where the toolchain does not have it yet.
    run("rustup", &["target", "add", one.target])?;
    let mut args = with.words().to_vec();
    args.extend([
        "--release",
        "--locked",
        "-p",
        "md2html",
        "--target",
        one.target,
    ]);
    run("cargo", &args).with_context(|| format!("building {} needs {}", one.name, with.needs()))?;
    Ok(target_dir()
        .join(one.target)
        .join("release")
        .join(one.binary()))
}

/// Checks a built binary: the right kind of file, and, where it can run here,
/// this version.
///
/// # Errors
///
/// The binary cannot be read (`was not built`); it is not the kind of
/// executable the platform runs (the message names the kind found and the
/// kind wanted); or, on this machine's own platform, it cannot be started
/// (`does not run`), prints output that is not UTF-8 (from
/// [`stdout_text`]), exits with a failure or answers with another version
/// (the message quotes its answer and the version wanted).
fn check(one: &Platform, binary: &Path) -> Result<()> {
    let bytes =
        std::fs::read(binary).with_context(|| format!("{} was not built", binary.display()))?;
    let kind = executable_kind(&bytes);
    if kind != Some(one.kind()) {
        bail!(
            "{} is {}, not the {} file {} runs",
            binary.display(),
            kind.unwrap_or("no executable"),
            one.kind(),
            one.name
        );
    }
    if is_this_machine(one) {
        let output = Command::new(binary)
            .arg("--version")
            .output()
            .with_context(|| format!("{} does not run", binary.display()))?;
        let succeeded = output.status.success();
        let command = format!("{} --version", binary.display());
        let answer = stdout_text(&command, output.stdout)?.trim().to_owned();
        if !succeeded || !answer.ends_with(VERSION) {
            bail!("{} answers `{answer}`, not {VERSION}", binary.display());
        }
    }
    Ok(())
}

/// Packs a folder of `dist` into a zip of the same name, with `zip`, or with
/// the `tar` that writes zips where there is no `zip` (Windows).
///
/// # Errors
///
/// Neither `zip -rqX` nor `tar -a -cf` runs and exits with success; the
/// message names the archive and says neither wrote it.
fn zip(dist: &Path, folder: &str) -> Result<PathBuf> {
    let name = format!("{folder}.zip");
    let archive = dist.join(&name);
    let packed = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .current_dir(dist)
            .status()
            .is_ok_and(|status| status.success())
    };
    if packed("zip", &["-rqX", &name, folder]) || packed("tar", &["-a", "-cf", &name, folder]) {
        Ok(archive)
    } else {
        Err(anyhow!(
            "{name} could not be packed: neither `zip` nor `tar -a` wrote it"
        ))
    }
}

/// Stages a platform's folder in `dist`, packs it, and returns the archive.
///
/// # Errors
///
/// A step on the files fails: removing an earlier staging folder, making
/// it, copying the binary, the README, the changelog or a licence into it,
/// removing an earlier archive, or removing the staging folder after packing;
/// the message names the path and the I/O error. Or the folder cannot be packed
/// (from [`zip`]).
fn pack(one: &Platform, binary: &Path, dist: &Path) -> Result<PathBuf> {
    let folder = format!("md2html-{}", one.name);
    let staged = dist.join(&folder);
    let named = |what: &Path| what.display().to_string();
    if staged.exists() {
        std::fs::remove_dir_all(&staged).with_context(|| named(&staged))?;
    }
    std::fs::create_dir_all(&staged).with_context(|| named(&staged))?;
    std::fs::copy(binary, staged.join(one.binary())).with_context(|| named(binary))?;
    for name in BESIDE {
        let from = workspace_root().join(name);
        std::fs::copy(&from, staged.join(name)).with_context(|| named(&from))?;
    }
    let old = dist.join(format!("{folder}.zip"));
    if old.exists() {
        std::fs::remove_file(&old).with_context(|| named(&old))?;
    }
    let archive = zip(dist, &folder)?;
    std::fs::remove_dir_all(&staged).with_context(|| named(&staged))?;
    Ok(archive)
}

/// Writes `CHECKSUMS.txt` and `SHA256SUMS` for every archive in `dist`, and
/// returns the lines.
///
/// # Errors
///
/// `dist` cannot be listed, an archive's name is not UTF-8 (the checksum
/// files name it as text), an archive cannot be read, or `CHECKSUMS.txt` or
/// `SHA256SUMS` cannot be written; the message names `dist` or the archive,
/// and the I/O error.
fn checksums(dist: &Path) -> Result<Vec<String>> {
    let named = || dist.display().to_string();
    let mut archives: Vec<(String, PathBuf)> = Vec::new();
    for listed in std::fs::read_dir(dist).with_context(named)? {
        let path = listed.with_context(named)?.path();
        if path.extension().is_none_or(|extension| extension != "zip") {
            continue;
        }
        // The checksum files name each archive as text.
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{}: the name is not UTF-8", path.display()))?
            .to_owned();
        archives.push((name, path));
    }
    archives.sort();
    let mut lines = Vec::new();
    for (name, archive) in archives {
        let bytes = std::fs::read(&archive).with_context(|| archive.display().to_string())?;
        lines.push(format!("{}  {name}", sha256_hex(&bytes)));
    }
    let text = format!("{}\n", lines.join("\n"));
    for name in ["CHECKSUMS.txt", "SHA256SUMS"] {
        std::fs::write(dist.join(name), &text).with_context(named)?;
    }
    Ok(lines)
}

/// Builds, checks and packs the platforms asked for; `Ok(false)` when one
/// failed.
///
/// # Errors
///
/// `dist` cannot be cleared (when every platform is built) or made, the
/// checksums cannot be written (from [`checksums`]), or standard output
/// cannot be written (from [`say`]). A platform that does not build, check
/// or pack is no error: it is reported with `!!` and makes the result
/// `Ok(false)`.
fn release(platforms: &[Platform], all: bool) -> Result<bool> {
    let dist = workspace_root().join("dist");
    let named = || dist.display().to_string();
    // Every platform at once is a fresh set: nothing of an earlier version
    // stays.
    if all && dist.exists() {
        std::fs::remove_dir_all(&dist).with_context(named)?;
    }
    std::fs::create_dir_all(&dist).with_context(named)?;
    let mut report = Vec::new();
    let mut failed = false;
    for one in platforms {
        let Some(with) = driver(std::env::consts::OS, one) else {
            report.push(format!(
                "-  {:<14} skipped: only a Mac builds for macOS",
                one.name
            ));
            continue;
        };
        say(format_args!("== {} ({})", one.name, one.target))?;
        let built = build(one, with)
            .and_then(|binary| check(one, &binary).map(|()| binary))
            .and_then(|binary| pack(one, &binary, &dist));
        match built {
            Ok(archive) => {
                let size = std::fs::metadata(&archive)
                    .with_context(|| archive.display().to_string())?
                    .len();
                report.push(format!(
                    "ok {:<14} {:<28} {}",
                    one.name,
                    one.target,
                    human_size(size)
                ));
            }
            Err(why) => {
                failed = true;
                report.push(format!("!! {:<14} {why:#}", one.name));
            }
        }
    }
    let sums = checksums(&dist)?;
    say(format_args!("\nmd2html {VERSION}, in {}:", dist.display()))?;
    for line in report.iter().chain(&sums) {
        say(format_args!("  {line}"))?;
    }
    Ok(!failed)
}

/// The arguments, as text.
///
/// # Errors
///
/// An argument is not valid Unicode: no platform has such a name, and the
/// message shows it.
fn arguments() -> Result<Vec<String>> {
    std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string().map_err(|bytes| {
                anyhow!(
                    "{} is not valid Unicode; see --list",
                    Path::new(&bytes).display()
                )
            })
        })
        .collect()
}

fn main() -> ExitCode {
    let done = match arguments().and_then(|args| asked(&args)) {
        Ok(Asked::Help) => say(format_args!(
            "usage: just release-local [--list] [PLATFORM ...]\n\
             The release archives for every platform this machine can build, in dist/."
        ))
        .map(|()| true)
        .map_err(anyhow::Error::from),
        Ok(Asked::List) => list().map(|()| true),
        Ok(Asked::Build { platforms, all }) => release(&platforms, all),
        Err(why) => Err(why),
    };
    match done {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        // The chain on one line: what was being done, then each cause.
        Err(why) => fail("release-local", &format!("{why:#}")),
    }
}
