// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Subprocess execution primitives for the MD2HTML CI pipeline.
//!
//! * [`execute_command`] / [`execute_command_with_env`] - run a single
//!   subprocess under the pipeline's timeout, logging, and progress-line
//!   conventions.
//! * [`execute_parallel`] - fan out a `Vec<(name, cmd, args)>` over
//!   `fanout_concurrency` worker threads, starting nothing new after the first
//!   failure.
//! * [`execute_step_with_tracking`] - adapter that wraps a `FnOnce() ->
//!   Result<()>` in the resumable-workflow tracking contract (mark-started →
//!   run → mark-completed/failed + record duration).

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::time::Duration;
use std::fs::OpenOptions;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Instant;

use anyhow::{Context as _, Result, anyhow, bail};

use crate::child::{output_within, panic_message};
use crate::context::PipelineContext;
use crate::out::{raw_err, say};
use crate::style::Paint as _;
use crate::workflow::WorkflowState;

// ─────────────────────────────────────────────────────────────────────────────
// Single-command execution
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn `cmd` with the given `args` under the pipeline's timeout +
/// logging conventions, injecting both the context-wide `global_env`
/// and the per-call `env` overrides.  On non-verbose runs the captured
/// output is tee'd to `ctx.log_file`; stderr of failed commands is
/// re-printed so the diagnosis is visible even without `--verbose`.
pub(crate) fn execute_command_with_env(
    name: &str,
    cmd: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
    ctx: &PipelineContext,
) -> Result<()> {
    let step_start = Instant::now();
    if ctx.flags.verbose {
        let env_text = env_vars
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        say!(
            "{} {} → {} {} (env: {env_text})",
            "→".blue().bold(),
            name.cyan(),
            cmd.yellow(),
            args.join(" ").dimmed(),
        )?;
    } else {
        say!("{} {}", "→".blue().bold(), name.cyan())?;
    }

    let mut command = Command::new(cmd);
    command.args(args);

    // Apply global environment variables first
    for (key, value) in &ctx.global_env {
        command.env(key, value);
    }

    // Then apply step-specific environment variables (can override globals)
    for (key, value) in env_vars {
        command.env(key, value);
    }

    // NOTE: the `CARGO_INCREMENTAL=0` ↔ `rustc-wrapper=sccache` pairing
    // is now enforced at the Cargo-config layer (`.cargo/config.toml`
    // sets both `build.incremental = false` and `build.rustc-wrapper =
    // "sccache"` as of Phase 3 of dev-flow-implementation-plan.md §
    // 2.1).  The pipeline still re-asserts `RUSTC_WRAPPER=sccache` in
    // its global env so that `git` (whose pre-push hook shells out to
    // cargo but reads no Cargo config itself) inherits the same
    // wrapper.

    // In verbose mode, inherit stdio; otherwise capture to log file
    if ctx.flags.verbose {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let Some(result) = output_within(&mut command, ctx.timeout_duration)
        .with_context(|| format!("Failed to run command '{cmd}' for step '{name}'"))?
    else {
        bail!(
            "Command '{}' timed out after {}s",
            cmd,
            ctx.timeout_duration.as_secs()
        );
    };
    let duration = step_start.elapsed();

    if let Some(log_path) = &ctx.log_file {
        let entry = LogEntry {
            name,
            cmd,
            args,
            duration,
            output: &result,
        };
        entry
            .append_to(log_path)
            .with_context(|| format!("writing the log file {}", log_path.display()))?;
    }

    if result.status.success() {
        say!("{} {} ({}s)", "✅".green(), name, duration.as_secs())?;
        Ok(())
    } else {
        let exit_code = result
            .status
            .code()
            .map_or_else(|| "unknown".to_owned(), |code| code.to_string());
        say!("{} {} failed (exit code: {})", "❌".red(), name, exit_code)?;

        // Print stderr on failure even in non-verbose mode
        if !ctx.flags.verbose && !result.stderr.is_empty() {
            raw_err(&result.stderr).with_context(|| {
                format!("Step '{name}' failed and its error output could not be shown")
            })?;
        }

        bail!(
            "Step '{}' failed: command '{}' exited with code {} after {}s",
            name,
            cmd,
            exit_code,
            duration.as_secs()
        );
    }
}

/// Thin wrapper around [`execute_command_with_env`] for callers that
/// don't need per-call env overrides beyond the context's global set.
pub(crate) fn execute_command(
    name: &str,
    cmd: &str,
    args: &[&str],
    ctx: &PipelineContext,
) -> Result<()> {
    execute_command_with_env(name, cmd, args, &[], ctx)
}

/// What one finished command leaves in the log file of a quiet run.
struct LogEntry<'run> {
    /// The step's name.
    name: &'run str,
    /// The program run.
    cmd: &'run str,
    /// Its arguments.
    args: &'run [&'run str],
    /// How long it took.
    duration: Duration,
    /// What it printed.
    output: &'run Output,
}

impl LogEntry<'_> {
    /// Appends the entry to the log file at `path`, creating the file.
    fn append_to(&self, path: &Path) -> io::Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "\n=== {} ({}) ===", self.name, self.cmd)?;
        writeln!(file, "Command: {} {}", self.cmd, self.args.join(" "))?;
        writeln!(file, "Duration: {}s", self.duration.as_secs())?;
        if !self.output.stdout.is_empty() {
            writeln!(file, "--- stdout ---")?;
            file.write_all(&self.output.stdout)?;
        }
        if !self.output.stderr.is_empty() {
            writeln!(file, "--- stderr ---")?;
            file.write_all(&self.output.stderr)?;
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Parallel fan-out
// ─────────────────────────────────────────────────────────────────────────────

/// Fan out every `(name, cmd, args)` tuple over at most
/// `ctx.fanout_concurrency` worker threads, with `env_vars` applied to
/// each.  After the first failure no further command starts; the ones
/// already running are waited for.  Used by the parallel-validation
/// stage and the security audit.
///
/// # Errors
///
/// A command that failed, with its name.
pub(crate) fn execute_parallel(
    commands: &[(&str, &str, Vec<&str>)],
    env_vars: &[(&str, &str)],
    ctx: &PipelineContext,
) -> Result<()> {
    let parallel_start = Instant::now();
    let command_count = commands.len();
    if env_vars.is_empty() {
        say!(
            "{} Running {} commands in parallel...",
            "🔄".yellow(),
            command_count
        )?;
    } else {
        say!(
            "{} Running {} commands in parallel with env vars...",
            "⚡".yellow(),
            command_count
        )?;
    }

    let queue = Queue {
        commands,
        next: AtomicUsize::new(0),
        failed: AtomicBool::new(false),
    };
    let workers = ctx.fanout_concurrency.min(command_count);
    thread::scope(|scope| {
        let handles: Vec<_> =
            core::iter::repeat_with(|| scope.spawn(|| queue.drain(env_vars, ctx)))
                .take(workers)
                .collect();
        handles.into_iter().try_for_each(|handle| {
            handle.join().map_err(|payload| {
                anyhow!("a parallel worker panicked: {}", panic_message(&*payload))
            })?
        })
    })
    .with_context(|| {
        format!("Parallel execution failed - one or more of {command_count} commands failed")
    })?;
    say!(
        "{} Parallel execution completed ({}s)",
        "✅".green(),
        parallel_start.elapsed().as_secs()
    )?;
    Ok(())
}

/// The commands of one [`execute_parallel`] call, shared by its workers.
struct Queue<'run> {
    /// Every command, in the order they are handed out.
    commands: &'run [(&'run str, &'run str, Vec<&'run str>)],
    /// The index of the next command to hand out.
    next: AtomicUsize,
    /// Set by the first worker whose command fails.
    failed: AtomicBool,
}

impl Queue<'_> {
    /// One worker: runs commands from the queue until it is empty or a
    /// command (here or on another worker) has failed.
    fn drain(&self, env_vars: &[(&str, &str)], ctx: &PipelineContext) -> Result<()> {
        while !self.failed.load(Ordering::Relaxed) {
            let Some((name, cmd, args)) =
                self.commands.get(self.next.fetch_add(1, Ordering::Relaxed))
            else {
                break;
            };
            if let Err(error) = execute_command_with_env(name, cmd, args, env_vars, ctx) {
                self.failed.store(true, Ordering::Relaxed);
                return Err(error.context(format!("Parallel execution failed for '{name}'")));
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Resumable-workflow step adapter
// ─────────────────────────────────────────────────────────────────────────────

/// Run `step_fn` under the resumable-workflow tracker.  Skips if the
/// step is already marked completed in `state`, otherwise records
/// started → completed / failed transitions around the call.  The
/// duration is recorded in `state.step_durations_secs` for per-run
/// performance comparison.
///
/// # Errors
///
/// Propagates the step's error verbatim, after recording the
/// failure into `state` via [`WorkflowState::mark_step_failed`].  Any
/// failure from the state-file write surfaces as a separate error.
pub(crate) fn execute_step_with_tracking<Step>(
    state: &mut WorkflowState,
    step_name: &str,
    step_fn: Step,
) -> Result<()>
where
    Step: FnOnce() -> Result<()>,
{
    if state.is_step_completed(step_name) {
        say!("⏭️  Skipping completed step: {step_name}")?;
        return Ok(());
    }
    state.mark_step_started(step_name)?;

    let step_start = Instant::now();
    let result = step_fn();
    let duration_secs = step_start.elapsed().as_secs();

    // Record step duration regardless of success/failure
    state
        .step_durations_secs
        .insert(step_name.to_owned(), duration_secs);

    match result {
        Ok(()) => {
            state.mark_step_completed(step_name)?;
            Ok(())
        }
        Err(err) => {
            state.mark_step_failed(step_name, &err.to_string())?;
            Err(err)
        }
    }
}
