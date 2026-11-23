// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! What a tool tells the person running it: a line of progress, a size as
//! a person reads it, and the end of a tool that stopped.

use std::io::Write as _;

/// Writes one line for the person running the tool.
///
/// # Errors
///
/// Standard output is closed or cannot be written (`... | head` that has
/// read enough, a full disk).
pub fn say(line: core::fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(std::io::stdout().lock(), "{line}")
        .map_err(|error| format!("standard output could not be written: {error}"))
}

/// Ends a tool that stopped: names the tool and why on standard error, and
/// gives the exit code for a failure.
#[must_use = "a tool that stopped must end with this exit code, or it ends as if it succeeded"]
pub fn fail(tool: &str, why: &str) -> std::process::ExitCode {
    // With standard error closed there is no one left to tell; the exit
    // code says it failed either way.
    match writeln!(std::io::stderr().lock(), "{tool}: {why}") {
        Ok(()) | Err(_) => std::process::ExitCode::FAILURE,
    }
}

/// A size in bytes the way a person reads it (`1.9 MB`).
#[must_use]
pub fn human_size(bytes: u64) -> String {
    let tenths_of_mb = bytes.saturating_mul(10) / (1024 * 1024);
    if tenths_of_mb >= 10 {
        format!("{}.{} MB", tenths_of_mb / 10, tenths_of_mb % 10)
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

#[cfg(test)]
mod tests {
    use super::human_size;

    /// Sizes read in whole kilobytes below a megabyte, in tenths above.
    ///
    /// # Panics
    ///
    /// When one byte is not `1 KB`, 900 KiB is not `900 KB`, 1 MiB is not
    /// `1.0 MB`, or 1.9 MiB is not rounded down to `1.8 MB`.
    #[test]
    fn sizes_read_as_a_person_says_them() {
        assert_eq!(human_size(1), "1 KB");
        assert_eq!(human_size(900 * 1024), "900 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
        assert_eq!(human_size(1024 * 1024 * 19 / 10), "1.8 MB");
    }
}
