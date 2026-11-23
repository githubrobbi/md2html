// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! Where the workspace is, where cargo builds into, and how a step is run
//! in it.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace's folder: two above this crate's manifest.
#[must_use]
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// Where cargo puts what it builds: `CARGO_TARGET_DIR`, or `target` in the
/// workspace.
#[must_use]
pub fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root().join("target"), PathBuf::from)
}

/// Runs one step in the workspace's folder, its output going where this tool's
/// goes.
///
/// # Errors
///
/// The step could not be started (the program is not installed), or it ended
/// with a failure; the message names the step.
pub fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let step = format!("{program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .status()
        .map_err(|error| format!("`{step}` could not be started: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{step}` failed ({status})"))
    }
}

/// What `command` wrote to its standard output, as text.  Decoded strictly:
/// bytes that are not UTF-8 are an error naming the command, never
/// replaced and read as if they were what it said.
///
/// # Errors
///
/// `stdout` is not valid UTF-8.
pub fn stdout_text(command: &str, stdout: Vec<u8>) -> Result<String, String> {
    String::from_utf8(stdout)
        .map_err(|error| format!("`{command}` wrote output that is not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::workspace_root;

    /// The workspace root is the folder of the workspace manifest.
    ///
    /// # Panics
    ///
    /// When the folder `workspace_root` gives holds no `Cargo.toml`, or no
    /// `crates/md2html/Cargo.toml`.
    #[test]
    fn the_workspace_root_holds_the_workspace_manifest() {
        assert!(workspace_root().join("Cargo.toml").is_file());
        assert!(workspace_root().join("crates/md2html/Cargo.toml").is_file());
    }
}
