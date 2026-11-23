// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The ship pipeline's clean step: whether to run `cargo clean` before
//! validation (a pure policy over the flags, the toolchain fingerprint and
//! the disk), and the step that applies it.

use core::time::Duration;
use std::path::Path;
use std::{fs, io};

use anyhow::{Context as _, Result};

use crate::cli::CleanMode;
use crate::context::{
    PipelineContext, active_rustc_id, bytes_to_gib, dir_size_bytes, disk_free_bytes,
    get_cargo_target_dir,
};
use crate::exec::{execute_command_with_env, execute_step_with_tracking};
use crate::out::say;
use crate::workflow::{STEP_CLEAN_ARTIFACTS, WorkflowState};

/// Filename (under `CARGO_TARGET_DIR`) of the last toolchain's fingerprint.
///
/// The file records the `rustc -vV`
/// fingerprint of the toolchain that last built the cache.  The clean
/// step compares it against the active toolchain on every run: a nightly
/// bump in step 00 leaves the shared target dir built by the previous
/// rustc, whose cross-crate metadata is version-specific, so reusing it
/// explodes with `E0514`.  A mismatch forces a `cargo clean` before any
/// build step runs.
const RUSTC_FINGERPRINT_FILE: &str = ".md2html-ci-rustc-fingerprint";

/// Outcome of the clean-step decision.  Extracted as a pure value so the
/// side-effecting async step body stays a thin shell over the
/// unit-tested [`decide_clean`] policy.
enum CleanDecision {
    /// Run `cargo clean`; the payload is the human-readable reason.
    Clean(&'static str),
    /// Skip the clean - nothing is stale and the disk thresholds are fine.
    Skip,
    /// A toolchain change needs a clean but `--no-clean` suppressed it;
    /// the stale fingerprint is left in place so the next run re-detects
    /// the mismatch.
    SuppressedToolchainChange,
}

/// Pure clean-step policy.  Precedence, highest first:
///
/// 1. [`CleanMode::Force`] (`--clean`) always cleans.
/// 2. [`CleanMode::Never`] (`--no-clean`) suppresses everything (a pending
///    toolchain change is reported separately so its fingerprint can be
///    preserved).
/// 3. A toolchain (rustc) change forces a clean to avoid stale-artifact
///    `E0514`.
/// 4. Disk pressure (low free space or oversized target) auto-cleans.
const fn decide_clean(
    mode: CleanMode,
    toolchain_changed: bool,
    disk_pressure: bool,
) -> CleanDecision {
    match mode {
        CleanMode::Force => CleanDecision::Clean("Forced clean (--clean flag)"),
        CleanMode::Never => {
            if toolchain_changed {
                CleanDecision::SuppressedToolchainChange
            } else {
                CleanDecision::Skip
            }
        }
        CleanMode::Auto => {
            if toolchain_changed {
                CleanDecision::Clean(
                    "Toolchain changed (rustc fingerprint mismatch) - forcing clean to avoid stale-artifact E0514",
                )
            } else if disk_pressure {
                CleanDecision::Clean("Auto-clean triggered (disk space low or target too large)")
            } else {
                CleanDecision::Skip
            }
        }
    }
}

/// Return `true` when `target_dir` already holds build output.
///
/// That is any
/// entry other than the fingerprint file itself.  A missing or
/// fingerprint-only dir means there is nothing to invalidate, so a fresh
/// checkout (or the state right after a `cargo clean`) never triggers a
/// spurious toolchain-clean.
fn target_dir_has_build_output(target_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(target_dir) else {
        return false;
    };
    entries
        .flatten()
        .any(|entry| &*entry.file_name().to_string_lossy() != RUSTC_FINGERPRINT_FILE)
}

/// Persist the active `rustc` fingerprint under `target_dir`.
///
/// It records
/// the toolchain that (re)built the cache so a later run can detect the
/// next bump.  Recreates the dir if `cargo clean` removed it.  A marker
/// that is not written makes the next run see a toolchain change and
/// clean again, so a failed write is an error.
///
/// # Errors
///
/// `target_dir` cannot be created, or the fingerprint file in it cannot be
/// written; the error is the one the file system gave.
fn record_rustc_fingerprint(target_dir: &Path, id: &str) -> io::Result<()> {
    fs::create_dir_all(target_dir)?;
    fs::write(target_dir.join(RUSTC_FINGERPRINT_FILE), id)
}

/// Tracked clean step: run `cargo clean` when the toolchain or the disk asks.
///
/// It forces a `cargo clean` when the active `rustc`
/// differs from the toolchain that built the cached `target` dir (a
/// nightly bump → stale cross-crate metadata → `E0514`), and otherwise
/// applies the disk-pressure auto-clean policy.  `--clean` always cleans;
/// `--no-clean` suppresses both paths.  After the decision the active
/// toolchain fingerprint is recorded for the next run.
///
/// # Errors
///
/// `cargo clean` cannot be run, times out or fails (from
/// [`execute_command_with_env`]); the fingerprint cannot be recorded
/// (`recording the toolchain fingerprint in <dir>`); the workflow state
/// file cannot be written when the step starts, ends or fails; or
/// standard output cannot be written.  A failure inside the step is
/// recorded as the step's failure before it is returned.
pub(super) fn tracked_clean_step(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    execute_step_with_tracking(state, STEP_CLEAN_ARTIFACTS, || {
        let target_dir = get_cargo_target_dir();
        let free_gb = disk_free_bytes(&target_dir).map(bytes_to_gib);
        let target_gb = dir_size_bytes(&target_dir, Duration::from_secs(30)).map(bytes_to_gib);

        // Toolchain fingerprint: did rustc change since this target dir
        // was last built?  Only meaningful when the dir actually holds
        // build output and we could read the active toolchain id.
        let active_id = active_rustc_id();
        let stored_id = fs::read_to_string(target_dir.join(RUSTC_FINGERPRINT_FILE))
            .ok()
            .map(|text| text.trim().to_owned());
        let toolchain_changed = active_id.is_some()
            && target_dir_has_build_output(&target_dir)
            && stored_id.as_deref() != active_id.as_deref();

        if ctx.flags.verbose {
            if let Some(free) = free_gb {
                say!("  💾 Free disk space: {free} GiB")?;
            }
            if let Some(size) = target_gb {
                say!("  📁 Target directory size: {size} GiB")?;
            }
        }

        let disk_pressure = free_gb.is_some_and(|gb| gb < ctx.min_free_gb)
            || target_gb.is_some_and(|gb| gb > ctx.max_target_gb);

        match decide_clean(ctx.flags.clean, toolchain_changed, disk_pressure) {
            CleanDecision::Clean(reason) => {
                say!("  🧹 {reason}")?;
                // `cargo clean` doesn't compile anything but still probes
                // the toolchain via `<rustc-wrapper> rustc -vV`.  On some
                // macOS hosts sccache's wrapped probe dies with "Operation
                // not permitted" in nested subprocesses even when it works
                // at the top level.  Since clean never needs a wrapper,
                // force-clear `RUSTC_WRAPPER` for this specific step.
                execute_command_with_env(
                    "Clean build artifacts",
                    "cargo",
                    &["clean"],
                    &[("RUSTC_WRAPPER", "")],
                    ctx,
                )?;
            }
            CleanDecision::SuppressedToolchainChange => {
                say!(
                    "  ⚠️  rustc changed but --no-clean is set - keeping stale artifacts (build may hit E0514)"
                )?;
                // Leave the stale fingerprint untouched so the next run
                // re-detects the mismatch and can clean.
                return Ok(());
            }
            CleanDecision::Skip => {
                say!("  ⏭️  Skipping clean (disk OK, target size OK, toolchain unchanged)")?;
            }
        }

        // Record the toolchain that will build this target dir (after a
        // clean, or confirming the unchanged cache) for next-run detection.
        if let Some(id) = active_id {
            record_rustc_fingerprint(&target_dir, &id).with_context(|| {
                format!(
                    "recording the toolchain fingerprint in {}",
                    target_dir.display()
                )
            })?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{CleanDecision, CleanMode, decide_clean};

    /// `--clean` cleans even with no toolchain change and no disk pressure.
    ///
    /// # Panics
    ///
    /// When the decision is not a clean.
    #[test]
    fn force_mode_always_cleans() {
        assert!(matches!(
            decide_clean(CleanMode::Force, false, false),
            CleanDecision::Clean(_)
        ));
    }

    /// `--no-clean` holds back the clean a toolchain change asks for.
    ///
    /// # Panics
    ///
    /// When a toolchain change with disk pressure under `--no-clean` is not
    /// reported as a suppressed toolchain change.
    #[test]
    fn never_mode_suppresses_toolchain_change() {
        assert!(matches!(
            decide_clean(CleanMode::Never, true, true),
            CleanDecision::SuppressedToolchainChange
        ));
    }

    /// `--no-clean` skips the clean disk pressure alone asks for.
    ///
    /// # Panics
    ///
    /// When disk pressure without a toolchain change under `--no-clean` does
    /// not skip.
    #[test]
    fn never_mode_without_change_skips() {
        assert!(matches!(
            decide_clean(CleanMode::Never, false, true),
            CleanDecision::Skip
        ));
    }

    /// By default a toolchain change cleans.
    ///
    /// # Panics
    ///
    /// When a toolchain change without disk pressure does not clean.
    #[test]
    fn auto_mode_toolchain_change_forces_clean() {
        assert!(matches!(
            decide_clean(CleanMode::Auto, true, false),
            CleanDecision::Clean(_)
        ));
    }

    /// By default disk pressure cleans.
    ///
    /// # Panics
    ///
    /// When disk pressure without a toolchain change does not clean.
    #[test]
    fn auto_mode_disk_pressure_cleans() {
        assert!(matches!(
            decide_clean(CleanMode::Auto, false, true),
            CleanDecision::Clean(_)
        ));
    }

    /// By default a toolchain change with disk pressure cleans.
    ///
    /// # Panics
    ///
    /// When the two together do not clean.
    #[test]
    fn auto_mode_toolchain_change_takes_precedence_over_disk() {
        assert!(matches!(
            decide_clean(CleanMode::Auto, true, true),
            CleanDecision::Clean(_)
        ));
    }

    /// By default no toolchain change and no disk pressure skips the clean.
    ///
    /// # Panics
    ///
    /// When the decision is not a skip.
    #[test]
    fn auto_mode_nothing_to_do_skips() {
        assert!(matches!(
            decide_clean(CleanMode::Auto, false, false),
            CleanDecision::Skip
        ));
    }
}
