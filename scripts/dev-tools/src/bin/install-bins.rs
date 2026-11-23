// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! `just use-local`: build md2html from this checkout, optimised, and put it
//! on the PATH.
//!
//! The binary goes to `~/bin` (or the folder `MD2HTML_BIN_DIR` names). It is
//! copied beside its place and renamed onto it, so a build that is running is
//! never half replaced, and the one installed is asked for its version to show
//! that it runs.
//!
//! A workspace binary, not a shell recipe, so that it does the same under
//! every shell `just` uses, on Windows as on a Mac.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, bail};
use md2html_dev_tools::{VERSION, fail, run, say, stdout_text, target_dir};

/// The binary's file name on this machine.
const BINARY: &str = if cfg!(windows) {
    "md2html.exe"
} else {
    "md2html"
};

/// The folder the binary is installed to: `MD2HTML_BIN_DIR`, or `bin` in the
/// home folder.
///
/// # Errors
///
/// `MD2HTML_BIN_DIR` is unset or empty and neither `HOME` nor `USERPROFILE`
/// is set: the message says no home folder is known and to name one with
/// `MD2HTML_BIN_DIR`.
fn bin_dir() -> Result<PathBuf> {
    if let Some(named) = std::env::var_os("MD2HTML_BIN_DIR").filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(named));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join("bin"))
        .context("no home folder is known; name one with MD2HTML_BIN_DIR")
}

/// Copies `built` beside `installed` and renames it onto it.
///
/// # Errors
///
/// The copy beside `installed` cannot be written (`could not be written`,
/// naming the copy), or the rename onto `installed` is refused (`could not
/// be replaced`). After a refused rename the copy is removed; when that
/// fails too, the message adds that the copy is left beside it.
fn replace(built: &Path, installed: &Path) -> Result<()> {
    let staged = installed.with_extension(format!("new-{}", std::process::id()));
    std::fs::copy(built, &staged)
        .with_context(|| format!("{} could not be written", staged.display()))?;
    if let Err(error) = std::fs::rename(&staged, installed) {
        // Leave nothing behind when the rename is refused; were that to fail
        // too, the staged copy is named in the message.
        let left = std::fs::remove_file(&staged)
            .err()
            .map_or_else(String::new, |_| {
                format!("; {} is left beside it", staged.display())
            });
        return Err(error)
            .with_context(|| format!("{} could not be replaced{left}", installed.display()));
    }
    Ok(())
}

/// What the installed binary answers to `--version`.
///
/// # Errors
///
/// The binary cannot be started (`does not run`), it exits with a failure
/// (`--version failed`, with its exit status), or what it prints is not
/// UTF-8 (from [`stdout_text`]).
fn version_of(installed: &Path) -> Result<String> {
    let output = Command::new(installed)
        .arg("--version")
        .output()
        .with_context(|| format!("{} does not run", installed.display()))?;
    if !output.status.success() {
        bail!(
            "{} --version failed ({})",
            installed.display(),
            output.status
        );
    }
    let command = format!("{} --version", installed.display());
    Ok(stdout_text(&command, output.stdout)?.trim().to_owned())
}

/// Whether a folder is on the PATH.
fn on_path(dir: &Path) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|entry| entry == dir))
}

/// Builds, installs and checks; the error names the step that stopped it.
///
/// # Errors
///
/// `cargo build --release --locked -p md2html` cannot be started or fails
/// (from [`run`]); there is no folder to install to (from [`bin_dir`]); the
/// folder cannot be made (`could not be made`); the binary cannot be put in
/// place (from [`replace`]) or asked for its version (from [`version_of`]);
/// its answer does not end in this checkout's version (`the build and this
/// checkout disagree`); or standard output cannot be written (from
/// [`say`]).
fn install() -> Result<()> {
    run("cargo", &[
        "build",
        "--release",
        "--locked",
        "-p",
        "md2html",
    ])?;
    let built = target_dir().join("release").join(BINARY);
    let dir = bin_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("{} could not be made", dir.display()))?;
    let installed = dir.join(BINARY);
    replace(&built, &installed)?;
    let answer = version_of(&installed)?;
    if !answer.ends_with(VERSION) {
        bail!(
            "{} says `{answer}`, not {VERSION}: the build and this checkout disagree",
            installed.display()
        );
    }
    say(format_args!(
        "installed {answer} to {}",
        installed.display()
    ))?;
    if !on_path(&dir) {
        say(format_args!("note: {} is not on the PATH", dir.display()))?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match install() {
        Ok(()) => ExitCode::SUCCESS,
        // The chain on one line: what was being done, then each cause.
        Err(why) => fail("use-local", &format!("{why:#}")),
    }
}
