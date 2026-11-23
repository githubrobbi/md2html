// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The pages a run makes: each input, and where its page goes.

use std::path::{Path, PathBuf};

use super::args::{Action, Args, Out};
use super::files::{unsafe_output, walk};
use crate::report::{Exit, Failure};

/// Where a page's Markdown comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// Standard input.
    Stdin,
    /// A file.
    File(PathBuf),
}

impl Source {
    /// The file, when it is one.
    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            Self::Stdin => None,
            Self::File(path) => Some(path),
        }
    }

    /// Its name in messages.
    pub(super) fn shown(&self) -> String {
        match self {
            Self::Stdin => String::from("<stdin>"),
            Self::File(path) => path.display().to_string(),
        }
    }
}

/// Where a page goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Dest {
    /// Standard output.
    Stdout,
    /// A file.
    File(PathBuf),
}

impl Dest {
    /// The file, when it is one.
    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            Self::Stdout => None,
            Self::File(path) => Some(path),
        }
    }
}

/// One page to make.
#[derive(Clone, Debug)]
pub(super) struct Job {
    /// The Markdown.
    pub(super) source: Source,
    /// The page.
    pub(super) out: Dest,
}

/// An input found, and the folder it was found in: a page keeps its
/// place under that folder when the pages go to another one.
struct Found {
    /// The Markdown.
    source: Source,
    /// The folder named on the command line, or the file's own.
    root: Option<PathBuf>,
}

/// The pages a run makes.
///
/// # Errors
///
/// An input that is not there or a folder without Markdown
/// ([`Exit::NoInput`]); an output md2html refuses to write, or several
/// pages for standard output ([`Exit::Usage`]).
pub(super) fn plan(args: &Args) -> Result<Vec<Job>, Failure> {
    let found = find(&args.inputs)?;
    let several = found.len() > 1;
    let mut jobs = Vec::with_capacity(found.len());
    for input in found {
        let out = destination(&args.out, &input, several);
        if let (Source::File(source), Dest::File(page)) = (&input.source, &out) {
            if let Some(reason) = unsafe_output(source, page) {
                return Err(Failure::new(
                    Exit::Usage,
                    format!("refusing to write {}: {reason}", page.display()),
                ));
            }
        }
        jobs.push(Job {
            source: input.source,
            out,
        });
    }
    let to_stdout = jobs.iter().filter(|job| job.out == Dest::Stdout).count();
    if to_stdout > 1 && args.action == Action::Write {
        return Err(Failure::new(
            Exit::Usage,
            String::from("several pages cannot all go to stdout; give -o a folder"),
        ));
    }
    Ok(jobs)
}

/// Every input the command line names, folders opened.
///
/// # Errors
///
/// With [`Exit::NoInput`] when a folder holds no Markdown file
/// (`<folder> holds no Markdown file`) or when an input is neither `-`, a
/// folder nor a file (`<input>: no such file`).
fn find(inputs: &[PathBuf]) -> Result<Vec<Found>, Failure> {
    let mut found = Vec::new();
    for input in inputs {
        if input.as_os_str() == "-" {
            found.push(Found {
                source: Source::Stdin,
                root: None,
            });
        } else if input.is_dir() {
            let mut files = Vec::new();
            walk(input, &mut files);
            if files.is_empty() {
                return Err(Failure::new(
                    Exit::NoInput,
                    format!("{} holds no Markdown file", input.display()),
                ));
            }
            found.extend(files.into_iter().map(|file| Found {
                source: Source::File(file),
                root: Some(input.clone()),
            }));
        } else if input.is_file() {
            found.push(Found {
                source: Source::File(input.clone()),
                root: input.parent().map(Path::to_path_buf),
            });
        } else {
            return Err(Failure::new(
                Exit::NoInput,
                format!("{}: no such file", input.display()),
            ));
        }
    }
    Ok(found)
}

/// Where the page of `input` goes.
fn destination(out: &Out, input: &Found, several: bool) -> Dest {
    match (out, &input.source) {
        (Out::Stdout, _) | (Out::Beside, Source::Stdin) => Dest::Stdout,
        (Out::Path(folder), Source::File(source)) if several || is_folder(folder) => {
            let under = input
                .root
                .as_ref()
                .and_then(|root| source.strip_prefix(root).ok())
                .map_or_else(
                    || PathBuf::from(source.file_name().unwrap_or_default()),
                    Path::to_path_buf,
                );
            Dest::File(folder.join(under).with_extension("html"))
        }
        (Out::Path(page), _) => Dest::File(page.clone()),
        (Out::Beside, Source::File(source)) => Dest::File(source.with_extension("html")),
    }
}

/// Whether `-o` names a folder: one that exists, or a path ending in a
/// separator.
fn is_folder(path: &Path) -> bool {
    path.is_dir() || path.as_os_str().to_string_lossy().ends_with(['/', '\\'])
}
