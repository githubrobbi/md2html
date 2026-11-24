// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! What a tool tells the person running it: a line of progress, a size as
//! a person reads it, and the end of a tool that stopped.

use std::io::Write as _;

/// Writes one line for the person running the tool.
///
/// ```
/// md2html_dev_tools::say(format_args!("built {} of {}", 1, 6))?;
/// # Ok::<(), md2html_dev_tools::Unsaid>(())
/// ```
///
/// # Errors
///
/// [`Unsaid`], with the I/O error as its source, when standard output is
/// closed or cannot be written (`... | head` that has read enough, a full
/// disk).
pub fn say(line: core::fmt::Arguments<'_>) -> Result<(), Unsaid> {
    writeln!(std::io::stdout().lock(), "{line}").map_err(Unsaid)
}

/// A line that could not be written to standard output; the I/O error is
/// its [`source`](core::error::Error::source).
///
/// ```
/// fn report(done: usize) -> Result<(), md2html_dev_tools::Unsaid> {
///     md2html_dev_tools::say(format_args!("{done} archives written"))
/// }
/// report(6)?;
/// # Ok::<(), md2html_dev_tools::Unsaid>(())
/// ```
#[derive(Debug)]
pub struct Unsaid(std::io::Error);

impl core::fmt::Display for Unsaid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("standard output could not be written")
    }
}

impl core::error::Error for Unsaid {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// Ends a tool that stopped: names the tool and why on standard error, and
/// gives the exit code for a failure.
///
/// ```
/// use std::process::ExitCode;
///
/// fn main() -> ExitCode {
///     // Writes `release-local: no platform this machine can build` on
///     // standard error.
///     let code = md2html_dev_tools::fail("release-local", "no platform this machine can build");
///     assert_eq!(code, ExitCode::FAILURE);
///     ExitCode::SUCCESS
/// }
/// ```
#[must_use = "a tool that stopped must end with this exit code, or it ends as if it succeeded"]
pub fn fail(tool: &str, why: &str) -> std::process::ExitCode {
    // With standard error closed there is no one left to tell; the exit
    // code says it failed either way.
    match writeln!(std::io::stderr().lock(), "{tool}: {why}") {
        Ok(()) | Err(_) => std::process::ExitCode::FAILURE,
    }
}

/// A size in bytes the way a person reads it (`1.9 MB`).
///
/// ```
/// use md2html_dev_tools::human_size;
///
/// assert_eq!(human_size(1_500), "2 KB");
/// assert_eq!(human_size(1_530_944), "1.4 MB");
/// ```
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
