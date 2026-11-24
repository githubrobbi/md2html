// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! md2html as a person or a script runs it: the built program, started
//! with a command line in a folder of its own, judged by what
//! `docs/command-line.md` promises (the files it writes, what goes to
//! standard output and standard error, the exit code).
//!
//! The unit tests inside the crate call the command line's parts; these
//! run the program, so they hold what only the whole run does: the exit
//! code the shell sees, the streams, and a run that lasts.
//!
//! Everything here is test code, and says so to the compiler and to clippy.
#![cfg(test)]

mod exits;
mod lasting;
mod pages;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A folder of its own for one test, removed when the test is over.
struct Folder {
    /// The folder.
    dir: PathBuf,
}

impl Folder {
    /// A new, empty folder named for the test.
    ///
    /// # Panics
    ///
    /// When a folder of an earlier run cannot be removed, or the new one
    /// cannot be made in the temporary directory.
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("md2html-cli-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("an earlier run's folder removed");
        }
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        Self { dir }
    }

    /// Where it is.
    fn path(&self) -> &Path {
        &self.dir
    }

    /// Writes a file into it, the folders on its way made.
    ///
    /// # Panics
    ///
    /// When the file or a folder on its way cannot be written.
    fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a folder in the scratch folder");
        }
        std::fs::write(&path, contents).expect("a file in the scratch folder");
        path
    }

    /// A file in it, read as text.
    ///
    /// # Panics
    ///
    /// When the file cannot be read as UTF-8 text.
    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.join(name))
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    /// Whether a file or folder is in it.
    fn has(&self, name: &str) -> bool {
        self.dir.join(name).exists()
    }

    /// md2html, ready to run in this folder with `args`, in an environment
    /// of its own: a fixed date, no shared glossary, no Chrome.
    fn md2html(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_md2html"));
        command
            .args(args)
            .current_dir(&self.dir)
            .env("SOURCE_DATE_EPOCH", "1700000000")
            .env("MD2HTML_GLOSSARY", self.dir.join("no-shared-glossary.md"))
            .env_remove("MD2HTML_GLOSSARY_URL")
            .env_remove("MD2HTML_CHROME")
            .stdin(Stdio::null());
        command
    }

    /// Runs md2html here with `args` to its end.
    ///
    /// # Panics
    ///
    /// When the program cannot be started.
    fn run(&self, args: &[&str]) -> Ran {
        Ran::of(&mut self.md2html(args))
    }

    /// Runs md2html here with `args`, `input` on its standard input.
    ///
    /// The input is a file, not a pipe: a run that refuses its command
    /// line ends before it reads anything, and a write into a pipe nobody
    /// reads fails with a broken pipe, sooner or later as the two
    /// processes happen to be scheduled.
    ///
    /// # Panics
    ///
    /// When the input file cannot be made or opened, or the program
    /// started.
    fn run_with_input(&self, args: &[&str], input: &str) -> Ran {
        let given = self.write("standard-input.txt", input);
        let output = self
            .md2html(args)
            .stdin(Stdio::from(File::open(given).expect("the input opened")))
            .output()
            .expect("md2html starts");
        Ran::from(output)
    }

    /// Runs md2html here with `args`, its standard output a file it may
    /// only read, so that every write to it fails.
    ///
    /// # Panics
    ///
    /// When the file cannot be made or opened, or the program started.
    fn run_with_stdout_closed(&self, args: &[&str]) -> Ran {
        let shut = self.write("read-only-output", "");
        let output = self
            .md2html(args)
            .stdout(Stdio::from(File::open(shut).expect("opened to read")))
            .output()
            .expect("md2html starts");
        Ran::from(output)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.dir) {
            // A test that is failing already has its own message to show.
            assert!(
                std::thread::panicking(),
                "the scratch folder {} stays: {error}",
                self.dir.display()
            );
        }
    }
}

/// How a run ended, and what it said.
struct Ran {
    /// The exit code; `None` when a signal ended it.
    code: Option<i32>,
    /// Standard output.
    out: String,
    /// Standard error.
    err: String,
}

impl From<Output> for Ran {
    fn from(output: Output) -> Self {
        Self {
            code: output.status.code(),
            out: String::from_utf8(output.stdout).expect("standard output is UTF-8"),
            err: String::from_utf8(output.stderr).expect("standard error is UTF-8"),
        }
    }
}

impl Ran {
    /// Runs `command` to its end.
    ///
    /// # Panics
    ///
    /// When the program cannot be started.
    fn of(command: &mut Command) -> Self {
        Self::from(command.output().expect("md2html starts"))
    }

    /// Asserts the exit code, showing both streams when it is another.
    ///
    /// # Panics
    ///
    /// When the run did not end with `code`.
    fn ends_with(&self, code: i32) -> &Self {
        assert_eq!(
            self.code,
            Some(code),
            "stdout: {}\nstderr: {}",
            self.out,
            self.err
        );
        self
    }
}

/// A small page with a heading and a paragraph.
const NOTES: &str = "# Notes\n\nA paragraph.\n";
