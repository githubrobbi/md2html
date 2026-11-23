// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! Where the workspace is, where cargo builds into, and how a step is run
//! in it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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
/// [`StepError::NotStarted`] when the step could not be started (the program
/// is not installed), with the I/O error as its source;
/// [`StepError::Failed`] when it ended with a failure.
pub fn run(program: &str, args: &[&str]) -> Result<(), StepError> {
    let step = format!("{program} {}", args.join(" "));
    match Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(StepError::Failed { step, status }),
        Err(source) => Err(StepError::NotStarted { step, source }),
    }
}

/// Why a step [`run`] ran did not succeed; each names the step.
#[derive(Debug)]
pub enum StepError {
    /// It could not be started.
    NotStarted {
        /// The program and its arguments.
        step: String,
        /// Why it could not be started.
        source: std::io::Error,
    },
    /// It ended with a failure.
    Failed {
        /// The program and its arguments.
        step: String,
        /// How it ended.
        status: ExitStatus,
    },
}

impl core::fmt::Display for StepError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotStarted { step, source: _ } => write!(f, "`{step}` could not be started"),
            Self::Failed { step, status } => write!(f, "`{step}` failed ({status})"),
        }
    }
}

impl core::error::Error for StepError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::NotStarted { step: _, source } => Some(source),
            Self::Failed { step: _, status: _ } => None,
        }
    }
}

/// What `command` wrote to its standard output, as text.  Decoded strictly:
/// bytes that are not UTF-8 are an error naming the command, never
/// replaced and read as if they were what it said.
///
/// # Errors
///
/// [`NotUtf8`] when `stdout` is not valid UTF-8, with the decoding error as
/// its source.
pub fn stdout_text(command: &str, stdout: Vec<u8>) -> Result<String, NotUtf8> {
    String::from_utf8(stdout).map_err(|source| NotUtf8 {
        command: command.to_owned(),
        source,
    })
}

/// Output of a command that is not UTF-8.
#[derive(Debug)]
pub struct NotUtf8 {
    /// The command that wrote it.
    command: String,
    /// Where the bytes stop being UTF-8.
    source: alloc::string::FromUtf8Error,
}

impl core::fmt::Display for NotUtf8 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "`{}` wrote output that is not UTF-8", self.command)
    }
}

impl core::error::Error for NotUtf8 {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use core::error::Error as _;

    use super::{StepError, run, stdout_text, workspace_root};

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

    /// A step that cannot be started, one that fails and output that is not
    /// UTF-8 are errors that name the step and keep their cause.
    ///
    /// # Panics
    ///
    /// When a missing program is not `NotStarted` with the I/O error as its
    /// source, when a failing step is not `Failed`, or when output that is
    /// not UTF-8 is accepted or loses where it stops being UTF-8.
    #[test]
    fn a_failed_step_names_itself_and_keeps_its_cause() {
        let missing = run("md2html-no-such-program", &["x"]).unwrap_err();
        assert_eq!(
            missing.to_string(),
            "`md2html-no-such-program x` could not be started"
        );
        assert!(missing.source().is_some(), "the I/O error is kept");
        let failed = run("cargo", &["md2html-no-such-command"]).unwrap_err();
        assert!(
            matches!(failed, StepError::Failed { step: _, status: _ }),
            "{failed}"
        );
        let garbled = stdout_text("tool", b"ok \xff".to_vec()).unwrap_err();
        assert_eq!(garbled.to_string(), "`tool` wrote output that is not UTF-8");
        assert!(
            garbled
                .source()
                .is_some_and(|cause| cause.to_string().contains("index 3")),
            "where the bytes stop being UTF-8"
        );
    }
}
