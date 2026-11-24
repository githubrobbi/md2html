// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! md2html's standard output, written through a handle of its own.
//!
//! The standard library's `Stdout` takes a write the system refuses with
//! `EBADF` (a standard output opened only for reading) for a success, so
//! that a program writing a page into it ends as if the page had arrived.
//! The Rust md2html declares as its minimum (1.85) does, and every stable
//! release up to 1.100 with it; only later nightly releases report the
//! error. md2html writes instead through a handle of its own on what the
//! process was given (a duplicate of its descriptor on Linux and macOS, of
//! its handle on Windows), which reports every error the system gives,
//! whatever Rust built md2html. The bytes are the same either way.
//!
//! A terminal stays the standard library's: Windows' console takes text
//! only through it (it writes UTF-16 there), and a terminal is never a
//! file opened only for reading. A system that is neither Unix nor
//! Windows has no handle md2html can name, and writes through it too.
//!
//! The rule can go when md2html's minimum Rust is a release whose
//! standard library reports the error itself
//! (`docs/policies/platform_policy.md`).

use std::fs::File;
use std::io::{self, IsTerminal as _, Stdout, Write as _};

/// md2html's standard output, ready to take bytes.
///
/// Every write md2html makes to standard output goes through it (a test
/// holds that no other code names the standard library's).
#[derive(Debug)]
pub(crate) struct StandardOutput(Sink);

/// What the bytes go through.
#[derive(Debug)]
enum Sink {
    /// A handle of md2html's own on the process's standard output.
    Own(File),
    /// The standard library's: a terminal, or a system without handles
    /// md2html can name.
    Std(Stdout),
}

impl StandardOutput {
    /// The process's standard output.
    ///
    /// # Errors
    ///
    /// The system gives no handle on it: on Windows, a process started
    /// without a standard output.
    pub(crate) fn open() -> io::Result<Self> {
        if io::stdout().is_terminal() {
            return Ok(Self(Sink::Std(io::stdout())));
        }
        own_handle().map(Self)
    }

    /// Writes `bytes` whole and flushes them.
    ///
    /// # Errors
    ///
    /// The system's error when standard output cannot take them: a
    /// broken pipe after its reader closed, a file opened only for
    /// reading, a full disk.
    pub(crate) fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        match &mut self.0 {
            Sink::Own(file) => file.write_all(bytes),
            Sink::Std(stdout) => {
                let mut locked = stdout.lock();
                locked.write_all(bytes)?;
                locked.flush()
            }
        }
    }
}

/// A handle of md2html's own on standard output: its descriptor,
/// duplicated.
///
/// # Errors
///
/// The system's, when the descriptor cannot be duplicated.
#[cfg(unix)]
fn own_handle() -> io::Result<Sink> {
    use std::os::fd::AsFd as _;
    Ok(Sink::Own(File::from(
        io::stdout().as_fd().try_clone_to_owned()?,
    )))
}

/// A handle of md2html's own on standard output: its handle, duplicated.
///
/// # Errors
///
/// The system's, when the handle cannot be duplicated: a process started
/// without a standard output has none.
#[cfg(windows)]
fn own_handle() -> io::Result<Sink> {
    use std::os::windows::io::AsHandle as _;
    Ok(Sink::Own(File::from(
        io::stdout().as_handle().try_clone_to_owned()?,
    )))
}

/// The standard library's own: a system that is neither Unix nor Windows
/// has no handle md2html can name.
///
/// # Errors
///
/// None; the signature is the other systems'.
#[cfg(not(any(unix, windows)))]
fn own_handle() -> io::Result<Sink> {
    Ok(Sink::Std(io::stdout()))
}
