// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The command line: what the person running md2html asked for, and the
//! pages that makes. What md2html tells them goes through `report`.

mod args;
mod files;
mod live;
mod plan;
mod run;
#[cfg(test)]
mod tests;

use std::process::ExitCode;

#[cfg(test)]
pub(crate) use args::megabytes;
use args::{Action, Command, HELP, Mode};

use crate::report::{Exit, Reporter, Voice};

/// The usage line after an error on the command line.
const USAGE: &str = "usage: md2html [options] <input>... (md2html --help)";

/// Runs md2html with the process's command line.
pub(crate) fn main() -> ExitCode {
    let before_args = Reporter::new(Voice::Full);
    let args = match args::parse(std::env::args_os().skip(1)) {
        Ok(Command::Run(args)) => args,
        Ok(Command::Help) => {
            before_args.print(HELP);
            return before_args.finish(Exit::Done);
        }
        Ok(Command::Version) => {
            before_args.print(&format!("md2html {}\n", env!("CARGO_PKG_VERSION")));
            return before_args.finish(Exit::Done);
        }
        Err(message) => {
            before_args.error(format_args!("{message}\n{USAGE}"));
            return before_args.finish(Exit::Usage);
        }
    };
    let reporter = Reporter::new(match (args.quiet, args.action) {
        (false, _) => Voice::Full,
        (true, Action::Check) => Voice::Findings,
        (true, Action::Write) => Voice::Quiet,
    });
    let jobs = match plan::plan(&args) {
        Ok(jobs) => jobs,
        Err(failure) => {
            reporter.error(format_args!("{}", failure.message));
            return reporter.finish(failure.exit);
        }
    };
    let exit = match args.mode {
        Mode::Serve(port) => live::serve(&args, &jobs, port, &reporter),
        Mode::Once => run::render_all(&args, &jobs, &reporter),
        Mode::Watch => {
            // The first run's outcome is reported; the watch decides the end.
            run::render_all(&args, &jobs, &reporter);
            live::watch(&args, &jobs, &reporter)
        }
    };
    reporter.finish(exit)
}
