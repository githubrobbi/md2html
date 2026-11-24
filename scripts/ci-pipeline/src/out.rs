// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Where the pipeline's text for people goes, and how.
//!
//! Every line goes through [`say!`] (standard output) or [`raw_err`]
//! (standard error), on a locked handle, and a failed write is an error
//! the caller returns like any other: a closed pipe (`just go | head`)
//! ends the run with a message instead of a panic, and nothing is lost
//! without a word.

use std::io::{self, Write as _};

use anyhow::{Context as _, Result};

/// Writes one line of formatted text to standard output: `println!` with
/// the write error returned. Takes what `format!` takes, nothing for an
/// empty line.
///
/// A macro because `format_args!` exists only as a macro; it expands to
/// one call of [`line()`], whose `Result` the caller handles, so it hides no
/// control flow:
///
/// ```text
/// say!("{} of {} gates green", 31, 33)?;
/// say!()?;
/// ```
///
/// (The pipeline is a binary, whose examples rustdoc does not run; its
/// callers compile this shape.)
macro_rules! say {
    () => {
        $crate::out::line(format_args!(""))
    };
    ($($arg:tt)*) => {
        $crate::out::line(format_args!($($arg)*))
    };
}
pub(crate) use say;

/// What [`say!`] calls: `text` and a newline, on standard output.
///
/// # Errors
///
/// Standard output is closed or cannot be written.
pub(crate) fn line(text: core::fmt::Arguments<'_>) -> Result<()> {
    writeln!(io::stdout().lock(), "{text}").context("standard output could not be written")
}

/// Writes `bytes` as they are to standard error: what a failed command
/// printed there, shown unchanged.
///
/// # Errors
///
/// Standard error is closed or cannot be written.
pub(crate) fn raw_err(bytes: &[u8]) -> Result<()> {
    let mut err = io::stderr().lock();
    err.write_all(bytes)
        .and_then(|()| err.write_all(b"\n"))
        .context("standard error could not be written")
}
