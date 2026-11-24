// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Running a child process with a time limit.
//!
//! The standard library waits for a child without a limit; the pipeline
//! wants one (an hour for a step, half a minute for `du`, a minute for a
//! question to `git`, five for one that goes over the network).  Every
//! child the pipeline starts goes through [`output_within`], or through
//! [`captured`] and [`finished`] when running past the limit is an error.
//! The child's piped output is read on two threads while this one polls
//! for its exit, so a child that fills a pipe never stalls, and a child
//! past its limit is killed and waited for.  What a child printed is read as
//! text through one rule, [`stdout_text`]: strictly.

use core::any::Any;
use core::time::Duration;
use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::Instant;

/// How often a running child is asked whether it has exited.
const POLL: Duration = Duration::from_millis(50);

/// The time limit of a question asked on this machine: `git`, `rustc -vV`,
/// `df`, `sccache`, a tool's `--version`.
pub(crate) const LOCAL: Duration = Duration::from_secs(60);

/// The time limit of a command that goes over the network (`gh`,
/// `cargo update`).
pub(crate) const NETWORK: Duration = Duration::from_secs(300);

/// Runs `command` like [`Command::output`]: nothing on its standard input,
/// its standard output and error read; within `limit`.
///
/// # Errors
///
/// As [`finished`].
pub(crate) fn captured(command: &mut Command, limit: Duration) -> io::Result<Output> {
    finished(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        limit,
    )
}

/// Runs `command` with the streams it was given (by default this
/// process's own, like [`Command::status`]); its exit status and what it
/// wrote to the streams the caller piped.
///
/// # Errors
///
/// As [`output_within`], and an error of kind `TimedOut` naming the
/// program and the limit when it ran past `limit` and was killed.
pub(crate) fn finished(command: &mut Command, limit: Duration) -> io::Result<Output> {
    output_within(command, limit)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "`{}` ran past its limit of {limit:?} and was stopped",
                command.get_program().display(),
            ),
        )
    })
}

/// Runs `command` and returns its exit status and what it wrote to the
/// pipes the caller set up (`Stdio::piped()`; an inherited or null stream
/// comes back empty), or `None` when it ran past `limit` and was killed.
///
/// After a kill the output read so far is dropped with the child; the
/// readers still finish first, so a grandchild that keeps the pipe open
/// (a `rustc` under a killed `cargo`) holds the return until it exits.
///
/// # Errors
///
/// The child cannot be spawned, waited for or killed, or its output
/// cannot be read.
pub(crate) fn output_within(command: &mut Command, limit: Duration) -> io::Result<Option<Output>> {
    let mut child = command.spawn()?;
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    thread::scope(|scope| {
        let stdout_reader = scope.spawn(move || read_all(stdout_pipe));
        let stderr_reader = scope.spawn(move || read_all(stderr_pipe));
        let waited = wait_within(&mut child, limit);
        let stdout = joined(stdout_reader)?;
        let stderr = joined(stderr_reader)?;
        Ok(waited?.map(|status| Output {
            status,
            stdout,
            stderr,
        }))
    })
}

/// Waits for `child` until `limit` has passed since now; then kills it.
/// `None` means it was killed.
///
/// A limit too far off for the clock to hold is never reached: the child is
/// waited for until it exits.
///
/// # Errors
///
/// Asking the child whether it has exited fails, or, past the limit,
/// killing it or waiting for it after the kill fails; the error is the one
/// the operating system gave.
fn wait_within(child: &mut Child, limit: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now().checked_add(limit);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if deadline.is_some_and(|at| Instant::now() >= at) {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(POLL);
    }
}

/// What `command` wrote to its standard output, as text.
///
/// Output is read
/// strictly: bytes that are not UTF-8 are an error naming the command,
/// never replaced with U+FFFD and read as if they were what it said.
///
/// # Errors
///
/// `stdout` is not valid UTF-8.
pub(crate) fn stdout_text(command: &str, stdout: Vec<u8>) -> anyhow::Result<String> {
    String::from_utf8(stdout)
        .map_err(|error| anyhow::anyhow!("`{command}` wrote output that is not UTF-8: {error}"))
}

/// Everything `pipe` yields until it closes; nothing when there is no
/// pipe.
///
/// # Errors
///
/// Reading the pipe fails; the error is the one the read gave.
fn read_all<Pipe: Read>(pipe: Option<Pipe>) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(mut source) = pipe {
        source.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}

/// The bytes a reader thread collected; a reader that panicked is an
/// error that carries the panic's message.
///
/// # Errors
///
/// The reader thread panicked (`a child-output reader thread panicked:`
/// and the panic's message), or its read failed (that error, passed on).
fn joined(reader: thread::ScopedJoinHandle<'_, io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader.join().map_err(|payload| {
        io::Error::other(format!(
            "a child-output reader thread panicked: {}",
            panic_message(&*payload)
        ))
    })?
}

/// The message a panic carried (`panic!("...")` gives a `&str` or a
/// `String`); empty for any other payload.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::process::{Command, Stdio};

    use super::{captured, finished, output_within};

    /// A child that exits in time gives its exit code and both outputs.
    ///
    /// # Panics
    ///
    /// When `sh` cannot be run, when it is reported killed within a 30 second
    /// limit, or when the exit code is not 3 or the outputs are not `out` and
    /// `err`.
    #[test]
    fn a_quick_child_gives_its_status_and_output() {
        let output = output_within(
            Command::new("sh")
                .args(["-c", "printf out; printf err >&2; exit 3"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
            Duration::from_secs(30),
        )
        .expect("sh runs")
        .expect("within the limit");
        assert_eq!(output.status.code(), Some(3_i32));
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
    }

    /// A child that runs past its limit is killed.
    ///
    /// # Panics
    ///
    /// When `sleep` cannot be run, when a 30 second `sleep` under a 200 ms
    /// limit is reported as exited instead of killed, or when the kill takes 10
    /// seconds or more.
    #[test]
    fn a_child_past_its_limit_is_killed() {
        let started = std::time::Instant::now();
        let output = output_within(
            Command::new("sleep").arg("30").stdout(Stdio::null()),
            Duration::from_millis(200),
        )
        .expect("sleep runs");
        assert!(output.is_none(), "killed at the limit");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "did not wait it out"
        );
    }

    /// A command run like `output()` reads nothing from this process and
    /// gives back what it wrote; one past its limit is an error that names
    /// it and the limit.
    ///
    /// # Panics
    ///
    /// When `sh` or `sleep` cannot be run; when `cat` under [`captured`]
    /// does not end at once with nothing (its input is closed); or when a
    /// 30 second `sleep` under a 200 ms limit is not an error of kind
    /// `TimedOut` saying `` `sleep` ran past its limit of 200ms``, or takes 10
    /// seconds or more.
    #[test]
    fn a_child_past_its_limit_is_an_error_that_names_it() {
        let cat = captured(&mut Command::new("cat"), Duration::from_secs(30)).expect("cat runs");
        assert!(cat.status.success());
        assert!(cat.stdout.is_empty(), "nothing on its input");
        let started = std::time::Instant::now();
        let error = finished(
            Command::new("sleep").arg("30").stdout(Stdio::null()),
            Duration::from_millis(200),
        )
        .expect_err("killed at the limit");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert_eq!(
            error.to_string(),
            "`sleep` ran past its limit of 200ms and was stopped"
        );
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
