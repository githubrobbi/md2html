// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One run over the inputs: read each, render it, and write the page and
//! the copies asked for beside it, or with `--check` only report.

use std::path::Path;

use super::args::{Action, Args, Shape};
use super::files::{relative, write_atomic};
use super::output::{Exit, Reporter};
use super::plan::{Dest, Job, Source};
use crate::config::front_matter;
use crate::document::{Link, Neighbours, stem};
use crate::input::{read_file, read_limited};
use crate::{Rendered, export, render_doc};

/// What `--max-input` is called in a refusal.
const RAISE: &str = " (--max-input raises the limit)";

/// Renders every job; how the run ends.
pub(crate) fn render_all(args: &Args, jobs: &[Job], reporter: &Reporter) -> Exit {
    let with_nav = args.nav && jobs.len() > 1;
    let inputs: Vec<Input<'_>> = jobs
        .iter()
        .map(|job| Input::read(job, args.max_input, with_nav))
        .collect();
    let run = Run {
        args,
        inputs: &inputs,
        reporter,
    };
    let mut exit = Exit::Done;
    let mut problems = 0_usize;
    for (at, input) in inputs.iter().enumerate() {
        let made = match &input.text {
            Ok(markdown) => run.make(at, input.job, markdown),
            Err(message) => {
                reporter.error(format_args!("{message}"));
                Made::failed(Exit::NoInput)
            }
        };
        problems = problems.saturating_add(made.problems);
        if let Some(failed) = made.failed {
            exit = failed;
        }
    }
    if args.action == Action::Check && exit == Exit::Done {
        if problems > 0 {
            reporter.note(format_args!("md2html: {problems} to look at"));
            return Exit::Findings;
        }
        reporter.say(format_args!("md2html: nothing to fix"));
    }
    exit
}

/// A job with its Markdown read.
struct Input<'run> {
    /// The job.
    job: &'run Job,
    /// Its Markdown, or why it could not be read.
    text: Result<String, String>,
    /// Its title for the other pages' `--nav`; empty without one.
    title: String,
}

impl<'run> Input<'run> {
    /// Reads `job`'s Markdown, up to `max` bytes, and its title when the
    /// run links its pages.
    fn read(job: &'run Job, max: u64, with_nav: bool) -> Self {
        let text = read_source(&job.source, max);
        let title = match &text {
            Ok(markdown) if with_nav => quick_title(markdown, job.source.path()),
            Ok(_) | Err(_) => String::new(),
        };
        Self { job, text, title }
    }
}

/// The Markdown of a source, read up to `max` bytes.
pub(super) fn read_source(source: &Source, max: u64) -> Result<String, String> {
    match source {
        Source::Stdin => read_limited(std::io::stdin().lock(), max)
            .map_err(|unread| format!("stdin: {}", unread.describe(RAISE))),
        Source::File(path) => read_file(path, max)
            .map_err(|unread| format!("{}: {}", path.display(), unread.describe(RAISE))),
    }
}

/// A page's title without rendering it.
fn quick_title(markdown: &str, source: Option<&Path>) -> String {
    let (map, body, _) = front_matter(markdown);
    if let Some(title) = map.get("title") {
        return title.clone();
    }
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map_or_else(
            || stem(source),
            |title| title.trim().replace(['`', '*'], ""),
        )
}

/// What one job came to.
struct Made {
    /// How many things the author should look at.
    problems: usize,
    /// How it failed, if it did.
    failed: Option<Exit>,
}

impl Made {
    /// A job that failed before its page was rendered.
    const fn failed(exit: Exit) -> Self {
        Self {
            problems: 0,
            failed: Some(exit),
        }
    }
}

/// A run under way.
struct Run<'run> {
    /// The command line.
    args: &'run Args,
    /// Every page of the run, read.
    inputs: &'run [Input<'run>],
    /// Where to tell the person.
    reporter: &'run Reporter,
}

impl Run<'_> {
    /// Renders job number `at` from its Markdown and writes what it asks.
    fn make(&self, at: usize, job: &Job, markdown: &str) -> Made {
        let page = render_doc(
            markdown,
            job.source.path(),
            job.out.path(),
            &self.args.settings,
            self.args.shape == Shape::Fragment,
            self.neighbours(at, job),
        );
        let shown = job.source.shown();
        for warning in &page.warnings {
            self.reporter.warning(&shown, warning);
        }
        let problems = page.warnings.len();
        let failed = match (self.args.action, &job.out) {
            (Action::Check, _) => None,
            (Action::Write, Dest::Stdout) => self.to_stdout(&page),
            (Action::Write, Dest::File(out)) => self.to_file(&shown, out, &page),
        };
        Made { problems, failed }
    }

    /// The links to the pages before and after job `at`, with `--nav`.
    fn neighbours(&self, at: usize, job: &Job) -> Option<Neighbours> {
        if !self.args.nav || self.inputs.len() < 2 {
            return None;
        }
        // A neighbour is the input before or after this one; there is none
        // past either end.
        let link = |other: usize| -> Option<Link> {
            let neighbour = self.inputs.get(other)?;
            Some(Link {
                href: relative(job.out.path()?, neighbour.job.out.path()?),
                title: neighbour.title.clone(),
            })
        };
        Some(Neighbours {
            prev: at.checked_sub(1).and_then(link),
            next: at.checked_add(1).and_then(link),
        })
    }

    /// Writes the page to standard output.
    fn to_stdout(&self, page: &Rendered) -> Option<Exit> {
        let error = Reporter::page(page.html.as_bytes()).err()?;
        self.reporter
            .error(format_args!("writing the page to standard output: {error}"));
        Some(Exit::CantCreate)
    }

    /// Writes the page to `out`, the book and the PDF beside it, and says
    /// what was written.
    fn to_file(&self, shown: &str, out: &Path, page: &Rendered) -> Option<Exit> {
        if let Some(parent) = out.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.reporter
                    .error(format_args!("writing {}: {error}", out.display()));
                return Some(Exit::CantCreate);
            }
        }
        if let Err(reason) = write_atomic(out, page.html.as_bytes()) {
            self.reporter
                .error(format_args!("writing {}: {reason}", out.display()));
            return Some(Exit::CantCreate);
        }
        let mut failed = None;
        if let Some(bytes) = &page.epub {
            let book = out.with_extension("epub");
            match write_atomic(&book, bytes) {
                Ok(()) => self.reporter.say(format_args!(
                    "md2html: {} ({} bytes)",
                    book.display(),
                    bytes.len()
                )),
                Err(reason) => {
                    self.reporter
                        .error(format_args!("writing {}: {reason}", book.display()));
                    failed = Some(Exit::CantCreate);
                }
            }
        }
        if page.pdf {
            let pdf = out.with_extension("pdf");
            match export::pdf(out, &pdf) {
                Ok(()) => self
                    .reporter
                    .say(format_args!("md2html: {}", pdf.display())),
                Err(message) => {
                    self.reporter
                        .error(format_args!("{}: {message}", pdf.display()));
                    failed = Some(Exit::Software);
                }
            }
        }
        let warned = if page.warnings.is_empty() {
            String::new()
        } else {
            format!(", {} to look at", page.warnings.len())
        };
        self.reporter.say(format_args!(
            "md2html: {shown} -> {} ({} bytes, {} glossary terms, {} of them shared{warned})",
            out.display(),
            page.html.len(),
            page.terms,
            page.shared
        ));
        failed
    }
}
