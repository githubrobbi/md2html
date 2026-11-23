// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! How md2html talks to the person running it, and how a run ends.
//!
//! The command line and the live server share it, so it belongs to
//! neither.
//!
//! Every line md2html writes for a person goes through [`Reporter`]: the
//! summary and the help on standard output, warnings and errors on
//! standard error. A write that fails (`md2html a.md | head -c0` has
//! closed the pipe) does not stop the run and is not lost either: the run
//! ends with [`Exit::Io`] unless it already failed for a reason of its own.

use core::cell::Cell;
use core::fmt::Arguments;
use std::io::Write as _;
use std::process::ExitCode;

use crate::render::Warning;

/// How a run ends: the exit codes md2html uses, from `sysexits.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exit {
    /// Everything asked for was done.
    Done,
    /// `--check` found something to fix.
    Findings,
    /// The command line was wrong, or asked for something md2html refuses.
    Usage,
    /// An input could not be read, or there is none any more.
    NoInput,
    /// No port was free for `--serve`.
    Unavailable,
    /// A PDF could not be printed, or printing left something behind.
    Software,
    /// A page or a book could not be written.
    CantCreate,
    /// A message, or the server's listener, failed.
    Io,
}

impl Exit {
    /// The number the shell sees.
    const fn code(self) -> u8 {
        match self {
            Self::Done => 0,
            Self::Findings => 1,
            Self::Usage => 64,
            Self::NoInput => 66,
            Self::Unavailable => 69,
            Self::Software => 70,
            Self::CantCreate => 73,
            Self::Io => 74,
        }
    }
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        Self::from(exit.code())
    }
}

/// A run that stopped: how it ends and what to say about it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    /// The exit code.
    pub(crate) exit: Exit,
    /// What went wrong, without the `md2html: ` in front.
    pub(crate) message: String,
}

impl Failure {
    /// A failure with its exit code.
    pub(crate) const fn new(exit: Exit, message: String) -> Self {
        Self { exit, message }
    }
}

/// How much md2html says besides the pages and the errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Voice {
    /// The summary, the notes and every warning.
    Full,
    /// Only the warnings: `--quiet --check`, where they are the result.
    Findings,
    /// Only errors: `--quiet`.
    Quiet,
}

/// The stream a line goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stream {
    /// Standard output: the page, the summary, the help.
    Out,
    /// Standard error: warnings, notes and errors.
    Err,
}

/// The one place md2html writes to the person running it.
#[derive(Debug)]
pub(crate) struct Reporter {
    /// How much to say.
    voice: Voice,
    /// A line that could not be written, so the run can say it failed.
    lost: Cell<bool>,
}

impl Reporter {
    /// A reporter with this voice.
    pub(crate) const fn new(voice: Voice) -> Self {
        Self {
            voice,
            lost: Cell::new(false),
        }
    }

    /// Text on standard output, whatever the voice: the help, the version.
    pub(crate) fn print(&self, text: &str) {
        self.keep(Stream::Out, write_to(Stream::Out, text.as_bytes()));
    }

    /// The page itself on standard output.
    ///
    /// # Errors
    ///
    /// Standard output could not take it; the caller decides the exit.
    pub(crate) fn page(bytes: &[u8]) -> std::io::Result<()> {
        write_to(Stream::Out, bytes)
    }

    /// A line of the summary on standard output, unless quiet.
    pub(crate) fn say(&self, line: Arguments<'_>) {
        if self.voice == Voice::Full {
            self.line(Stream::Out, line);
        }
    }

    /// A note on standard error, unless quiet: what is being watched or
    /// served, how many findings there are.
    pub(crate) fn note(&self, line: Arguments<'_>) {
        if self.voice == Voice::Full {
            self.line(Stream::Err, line);
        }
    }

    /// One thing the author should look at, at its line of `shown`.
    pub(crate) fn warning(&self, shown: &str, warning: &Warning) {
        if self.voice == Voice::Quiet {
            return;
        }
        if warning.line > 0 {
            self.line(
                Stream::Err,
                format_args!("{shown}:{}: {}", warning.line, warning.message),
            );
        } else {
            self.line(Stream::Err, format_args!("{shown}: {}", warning.message));
        }
    }

    /// An error, always: `md2html: ` and what went wrong.
    pub(crate) fn error(&self, line: Arguments<'_>) {
        self.line(Stream::Err, format_args!("md2html: {line}"));
    }

    /// How the run ends: `exit`, or [`Exit::Io`] when a run that did
    /// everything else could not say so.
    #[must_use = "the exit code is how the run ends; dropped, a failed run ends as if it succeeded"]
    pub(crate) fn finish(&self, exit: Exit) -> ExitCode {
        if exit == Exit::Done && self.lost.get() {
            ExitCode::from(Exit::Io)
        } else {
            ExitCode::from(exit)
        }
    }

    /// Writes one line to `stream`.
    fn line(&self, stream: Stream, line: Arguments<'_>) {
        let mut text = String::new();
        crate::text::put!(text, "{line}\n");
        self.keep(stream, write_to(stream, text.as_bytes()));
    }

    /// Remembers that a write failed. The first time standard output
    /// fails, standard error says so once; when that fails too there is no
    /// one left to tell, and the exit code says it.
    fn keep(&self, stream: Stream, written: std::io::Result<()>) {
        let Err(error) = written else {
            return;
        };
        let first = !self.lost.replace(true);
        if first && stream == Stream::Out {
            self.line(
                Stream::Err,
                format_args!("md2html: standard output could not be written: {error}"),
            );
        }
    }
}

/// Writes `bytes` whole to `stream`, and flushes it.
///
/// # Errors
///
/// The error of the stream when it cannot take the bytes or be flushed,
/// for example a broken pipe after the reader of standard output closed.
fn write_to(stream: Stream, bytes: &[u8]) -> std::io::Result<()> {
    match stream {
        Stream::Out => {
            let mut out = std::io::stdout().lock();
            out.write_all(bytes)?;
            out.flush()
        }
        Stream::Err => {
            let mut err = std::io::stderr().lock();
            err.write_all(bytes)?;
            err.flush()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Exit;

    /// Each exit status keeps the number scripts rely on.
    ///
    /// # Panics
    ///
    /// When an [`Exit`] no longer gives its number from `sysexits.h` (64 for
    /// usage, 66 for no input, ...), or 0 for done and 1 for findings.
    #[test]
    fn exit_codes_keep_their_numbers() {
        let codes = [
            (Exit::Done, 0),
            (Exit::Findings, 1),
            (Exit::Usage, 64),
            (Exit::NoInput, 66),
            (Exit::Unavailable, 69),
            (Exit::Software, 70),
            (Exit::CantCreate, 73),
            (Exit::Io, 74),
        ];
        for (exit, code) in codes {
            assert_eq!(exit.code(), code, "{exit:?}");
        }
    }
}
