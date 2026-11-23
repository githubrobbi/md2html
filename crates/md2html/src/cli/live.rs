// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The runs that last: `--watch` renders again whenever an input changes,
//! `--serve` serves the pages on this computer and reloads them.

use alloc::collections::BTreeMap;
use core::time::Duration;
use std::path::{Path, PathBuf};

use super::args::Args;
use super::plan::Job;
use super::run::render_all;
use crate::document::shared_glossary_path;
use crate::input::read_file;
use crate::report::{Exit, Failure, Reporter};
use crate::watch::{Change, Watch};
use crate::{render_doc, serve};

/// How often `--watch` looks at the files.
const WATCH_EVERY: Duration = Duration::from_millis(300);

/// How often `--serve` looks at the files.
const SERVE_EVERY: Duration = Duration::from_millis(250);

/// The Markdown files of a run.
fn sources(jobs: &[Job]) -> Vec<PathBuf> {
    jobs.iter()
        .filter_map(|job| job.source.path().map(Path::to_path_buf))
        .collect()
}

/// What else a page is made from: the stylesheets and heads the command
/// line names, the shared glossary.
fn others(args: &Args) -> Vec<PathBuf> {
    args.settings
        .css
        .iter()
        .chain(&args.settings.head)
        .cloned()
        .chain(shared_glossary_path())
        .collect()
}

/// `--watch`: renders again whenever an input changes, until the inputs
/// are gone.
pub(crate) fn watch(args: &Args, jobs: &[Job], reporter: &Reporter) -> Exit {
    let inputs = sources(jobs);
    if inputs.is_empty() {
        reporter.error(format_args!("--watch needs files, not stdin"));
        return Exit::Usage;
    }
    let mut watch = Watch::new(&inputs, &others(args), WATCH_EVERY);
    reporter.note(format_args!(
        "md2html: watching {} file(s); Ctrl-C to stop",
        watch.len()
    ));
    let mut change = Change::Same;
    while change != Change::Gone {
        change = watch.wait();
        if change == Change::Changed {
            render_all(args, jobs, reporter);
        }
    }
    reporter.error(format_args!(
        "none of the inputs has been there for a while; stopped watching"
    ));
    Exit::NoInput
}

/// `--serve`: the pages rendered in memory and served on this computer
/// from `port` up, rendered again and reloaded in the browser on every
/// save.
pub(crate) fn serve(args: &Args, jobs: &[Job], port: u16, reporter: &Reporter) -> Exit {
    let inputs = sources(jobs);
    if inputs.is_empty() {
        reporter.error(format_args!("--serve needs files, not stdin"));
        return Exit::Usage;
    }
    // The folder served: the folder given, or the first file's.
    let root = args
        .inputs
        .iter()
        .find(|input| input.is_dir())
        .cloned()
        .or_else(|| {
            inputs
                .first()
                .and_then(|first| first.parent())
                .map(Path::to_path_buf)
        })
        .filter(|root| !root.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut settings = args.settings.clone();
    settings.serve = Some(true);
    settings.pdf = None;
    settings.epub = None;
    let render = || {
        let mut pages = BTreeMap::new();
        for src in &inputs {
            let markdown = match read_file(src, args.max_input) {
                Ok(markdown) => markdown,
                Err(unread) => {
                    reporter.error(format_args!(
                        "{}: {}",
                        src.display(),
                        unread.describe(" (--max-input raises the limit)")
                    ));
                    continue;
                }
            };
            let rel = src
                .strip_prefix(&root)
                .unwrap_or(src)
                .with_extension("html");
            let url = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
            let page = render_doc(
                &markdown,
                Some(src),
                Some(&root.join(&rel)),
                &settings,
                false,
                None,
            );
            let shown = src.display().to_string();
            for warning in &page.warnings {
                reporter.warning(&shown, warning);
            }
            pages.insert(url, page.html);
        }
        pages
    };
    let watch = Watch::new(&inputs, &others(args), SERVE_EVERY);
    match serve::run(&root, port, watch, render, reporter) {
        Ok(()) => Exit::Done,
        Err(Failure { exit, message }) => {
            reporter.error(format_args!("{message}"));
            exit
        }
    }
}
