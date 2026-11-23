// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Running a child process with a time limit.
//!
//! The standard library waits for a child without a limit; the pipeline
//! wants one (an hour for a step, half a minute for `du`).  The child's
//! piped output is read on two threads while this one polls for its exit,
//! so a child that fills a pipe never stalls, and a child past its limit
//! is killed.  What a child printed is read as text through one rule,
//! [`stdout_text`]: strictly.

use core::any::Any;
use core::time::Duration;
use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Output};
use std::thread;
use std::time::Instant;

/// How often a running child is asked whether it has exited.
const POLL: Duration = Duration::from_millis(50);

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
fn wait_within(child: &mut Child, limit: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(POLL);
    }
}

/// What `command` wrote to its standard output, as text.  Output is read
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
fn read_all<Pipe: Read>(pipe: Option<Pipe>) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(mut source) = pipe {
        source.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}

/// The bytes a reader thread collected; a reader that panicked is an
/// error that carries the panic's message.
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

    use super::output_within;

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
}
