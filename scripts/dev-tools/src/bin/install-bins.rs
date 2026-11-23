// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a developer's command-line tool: it reports to stdout and names the step that \
              stopped it on stderr"
)]

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

use md2html_dev_tools::{VERSION, run, target_dir};

/// The binary's file name on this machine.
const BINARY: &str = if cfg!(windows) {
    "md2html.exe"
} else {
    "md2html"
};

/// The folder the binary is installed to: `MD2HTML_BIN_DIR`, or `bin` in the
/// home folder.
fn bin_dir() -> Result<PathBuf, String> {
    if let Some(named) = std::env::var_os("MD2HTML_BIN_DIR").filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(named));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join("bin"))
        .ok_or_else(|| "no home folder is known; name one with MD2HTML_BIN_DIR".to_owned())
}

/// Copies `built` beside `installed` and renames it onto it.
fn replace(built: &Path, installed: &Path) -> Result<(), String> {
    let staged = installed.with_extension(format!("new-{}", std::process::id()));
    std::fs::copy(built, &staged)
        .map_err(|error| format!("{} could not be written: {error}", staged.display()))?;
    std::fs::rename(&staged, installed).map_err(|error| {
        // Leave nothing behind when the rename is refused; were that to fail
        // too, the staged copy is named in the message.
        let left = std::fs::remove_file(&staged)
            .err()
            .map_or_else(String::new, |_| {
                format!("; {} is left beside it", staged.display())
            });
        format!(
            "{} could not be replaced: {error}{left}",
            installed.display()
        )
    })
}

/// What the installed binary answers to `--version`.
fn version_of(installed: &Path) -> Result<String, String> {
    let output = Command::new(installed)
        .arg("--version")
        .output()
        .map_err(|error| format!("{} does not run: {error}", installed.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} --version failed ({})",
            installed.display(),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Whether a folder is on the PATH.
fn on_path(dir: &Path) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|entry| entry == dir))
}

/// Builds, installs and checks; the error names the step that stopped it.
fn install() -> Result<(), String> {
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
        .map_err(|error| format!("{} could not be made: {error}", dir.display()))?;
    let installed = dir.join(BINARY);
    replace(&built, &installed)?;
    let answer = version_of(&installed)?;
    if !answer.ends_with(VERSION) {
        return Err(format!(
            "{} says `{answer}`, not {VERSION}: the build and this checkout disagree",
            installed.display()
        ));
    }
    println!("installed {answer} to {}", installed.display());
    if !on_path(&dir) {
        println!("note: {} is not on the PATH", dir.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match install() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("use-local: {why}");
            ExitCode::FAILURE
        }
    }
}
