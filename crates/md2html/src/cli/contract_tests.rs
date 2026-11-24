// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The command line's contract, `docs/command-line.md`, held to the code.
//!
//! The page lists every option, setting, exit code and environment
//! variable md2html reads. These tests read the page's tables and the code
//! that reads each of them, and fail when one names what the other does
//! not: an option added to the parser and not to the page fails here, and
//! so does a setting the page lists and md2html does not know.

use alloc::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::args::{Command, HELP, parse};
use crate::config::{FAMILY, LANGS, Settings};
use crate::report::Exit;

/// The contract, from the crate's folder.
const PAGE: &str = "../../docs/command-line.md";

/// Every way a run ends, in the order of their numbers.
const EVERY_EXIT: [Exit; 8] = [
    Exit::Done,
    Exit::Findings,
    Exit::Usage,
    Exit::NoInput,
    Exit::Unavailable,
    Exit::Software,
    Exit::CantCreate,
    Exit::Io,
];

/// Where `exit` stands in [`EVERY_EXIT`]. The match has no wildcard, so a
/// new way for a run to end does not compile until it is placed here, and
/// then [`every_exit_is_listed`] asks for it in the list.
const fn place(exit: Exit) -> usize {
    match exit {
        Exit::Done => 0,
        Exit::Findings => 1,
        Exit::Usage => 2,
        Exit::NoInput => 3,
        Exit::Unavailable => 4,
        Exit::Software => 5,
        Exit::CantCreate => 6,
        Exit::Io => 7,
    }
}

/// A file of the repository, by its path from the crate's folder.
///
/// # Panics
///
/// When the file cannot be read.
fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The lines of the page's chapter `title` (a `## ` heading), up to the
/// next chapter.
fn chapter<'page>(page: &'page str, title: &str) -> Vec<&'page str> {
    let heading = format!("## {title}");
    page.lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .collect()
}

/// Whether a table line is the row under a table's head: `| --- | --- |`.
fn is_rule(line: &str) -> bool {
    line.starts_with('|') && line.chars().all(|ch| matches!(ch, '|' | '-' | ' ' | ':'))
}

/// What the first cell of each row of the tables in `lines` names in
/// backticks, row by row; the heads of the tables left out.
fn first_cell_rows(lines: &[&str]) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let head = lines.get(at + 1).is_some_and(|next| is_rule(next));
        if !line.starts_with('|') || is_rule(line) || head {
            continue;
        }
        let cell = line.split('|').nth(1).unwrap_or_default();
        rows.push(
            cell.split('`')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect(),
        );
    }
    rows
}

/// What the first cells of the tables in `lines` name in backticks; the
/// heads of the tables left out.
fn first_cells(lines: &[&str]) -> BTreeSet<String> {
    first_cell_rows(lines).into_iter().flatten().collect()
}

/// The words of a line of the help, split where an option's spelling
/// ends: at white space, a comma or a bracket.
fn help_words(line: &str) -> BTreeSet<&str> {
    line.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '(' | ')' | ';' | ':'))
        .filter(|word| !word.is_empty())
        .collect()
}

/// The string literals of `code`, comment lines left out. An escaped
/// character stays escaped; none of the literals read here has one.
fn literals(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in code
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
    {
        let mut chars = line.chars();
        while chars.by_ref().any(|ch| ch == '"') {
            let mut literal = String::new();
            let mut escaped = false;
            for ch in chars.by_ref() {
                if ch == '"' && !escaped {
                    break;
                }
                escaped = ch == '\\' && !escaped;
                literal.push(ch);
            }
            found.push(literal);
        }
    }
    found
}

/// Whether `word` is spelled like an option: `-o`, `--no-toc`.
fn is_option(word: &str) -> bool {
    let long = word
        .strip_prefix("--")
        .is_some_and(|name| name.starts_with(|ch: char| ch.is_ascii_lowercase()));
    let short = word.len() == 2
        && word
            .strip_prefix('-')
            .is_some_and(|name| name.chars().all(|ch| ch.is_ascii_alphabetic()));
    (long || short)
        && word
            .chars()
            .all(|ch| ch == '-' || ch.is_ascii_alphanumeric())
}

/// Every option the parser reads: the literals of `cli/args.rs` spelled
/// like one.
fn parsed_options() -> BTreeSet<String> {
    literals(&read("src/cli/args.rs"))
        .into_iter()
        .filter(|word| is_option(word))
        .collect()
}

/// Every key `Settings::set` reads: the literals of its match's arms, the
/// keys it passes over included.
///
/// # Panics
///
/// When `config/settings.rs` cannot be read, or `Settings::set` no longer
/// matches on `name.as_str()` up to an `other` arm that refuses the key.
fn set_keys() -> BTreeSet<String> {
    let code = read("src/config/settings.rs");
    let arms = code
        .split_once("match name.as_str() {")
        .and_then(|(_, rest)| rest.split_once("other => return Err"))
        .map(|(arms, _)| arms)
        .expect("`Settings::set` matches on `name.as_str()`");
    arms.lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('"') || line.starts_with("| \""))
        .flat_map(|line| literals(line.split_once("=>").map_or(line, |(keys, _)| keys)))
        .collect()
}

/// Every environment variable the production code reads.
///
/// Those it names to `std::env::var_os` or `std::env::var`, and the
/// system's temporary folder's (`TMPDIR`; `TMP` and `TEMP` on Windows)
/// when it asks `std::env::temp_dir`, which reads them.
///
/// # Panics
///
/// When a folder or a file of the sources cannot be read.
fn read_variables() -> BTreeSet<String> {
    let mut files = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut names = BTreeSet::new();
    while let Some(path) = files.pop() {
        if path.is_dir() {
            let listing = std::fs::read_dir(&path).expect("the source folder lists");
            files.extend(listing.map(|entry| entry.expect("a source entry").path()));
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let test_file = name == "tests.rs"
            || name.ends_with("_tests.rs")
            || path.components().any(|part| part.as_os_str() == "tests");
        let rust = path.extension().is_some_and(|extension| extension == "rs");
        if test_file || !rust {
            continue;
        }
        let code = std::fs::read_to_string(&path).expect("a source file reads");
        let production = code
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(code.as_str(), |(before, _)| before);
        for call in ["var_os(\"", "env::var(\""] {
            for (_, after) in production
                .match_indices(call)
                .map(|(at, _)| production.split_at(at))
            {
                names.extend(literals(after).into_iter().take(1));
            }
        }
        if production.contains("env::temp_dir()") {
            names.extend(["TMPDIR", "TMP", "TEMP"].map(str::to_owned));
        }
    }
    names
}

/// The command line `words`, parsed.
///
/// # Errors
///
/// What the parser refuses, as it says it.
fn parsed(words: &[&str]) -> Result<Command, String> {
    parse(words.iter().map(OsString::from))
}

/// The options of the page are the options of the parser, and the help
/// shows none the page does not list.
///
/// # Panics
///
/// When an option the parser reads is not on the page, the page lists one
/// the parser does not read or refuses as unknown, or the help names one
/// the page does not list.
#[test]
fn every_option_is_on_the_page_and_no_other() {
    let page = read(PAGE);
    let documented = first_cells(&chapter(&page, "Options"));
    let read_by_parser = parsed_options();
    assert!(
        read_by_parser.len() > 30,
        "the parser's options are found: {read_by_parser:?}"
    );
    let missing: Vec<_> = read_by_parser.difference(&documented).collect();
    assert!(missing.is_empty(), "options not on the page: {missing:?}");
    let unknown: Vec<_> = documented.difference(&read_by_parser).collect();
    assert!(
        unknown.is_empty(),
        "options on the page the parser does not read: {unknown:?}"
    );
    for option in &documented {
        let refused = parsed(&["in.md", option, "x"])
            .err()
            .filter(|message| message.starts_with("unknown option"));
        assert_eq!(refused, None, "{option}");
    }
    let shown: BTreeSet<&str> = help_words(HELP)
        .into_iter()
        .filter(|word| is_option(word))
        .collect();
    let undocumented: Vec<_> = shown
        .iter()
        .filter(|option| !documented.contains(**option))
        .collect();
    assert!(
        undocumented.is_empty(),
        "the help shows options the page does not list: {undocumented:?}"
    );
}

/// The settings of the page are the keys `Settings::set` reads.
///
/// # Panics
///
/// When a key `Settings::set` reads is not on the page, or the page lists
/// a key it does not read or reports as unknown.
#[test]
fn every_setting_is_on_the_page_and_no_other() {
    let page = read(PAGE);
    let documented = first_cells(&chapter(&page, "Settings"));
    let known = set_keys();
    assert!(known.len() > 40, "the settings' keys are found: {known:?}");
    let missing: Vec<_> = known.difference(&documented).collect();
    assert!(missing.is_empty(), "settings not on the page: {missing:?}");
    let unknown: Vec<_> = documented.difference(&known).collect();
    assert!(
        unknown.is_empty(),
        "settings on the page md2html does not read: {unknown:?}"
    );
    for key in &documented {
        let mut settings = Settings::default();
        let refused = ["yes", "native"]
            .iter()
            .map(|value| settings.set(key, value))
            .all(|set| set.is_err_and(|message| message.starts_with("unknown setting")));
        assert!(!refused, "{key} is a setting");
    }
}

/// [`EVERY_EXIT`] lists every way a run ends, each once.
///
/// # Panics
///
/// When an [`Exit`] stands elsewhere in the list than [`place`] says.
#[test]
fn every_exit_is_listed() {
    for (at, exit) in EVERY_EXIT.iter().enumerate() {
        assert_eq!(place(*exit), at, "{exit:?}");
    }
}

/// The exit codes of the page are md2html's, each once.
///
/// # Panics
///
/// When an exit code md2html uses is not in the page's table, the table
/// lists a code md2html does not use, or a code is not a number.
#[test]
fn every_exit_code_is_on_the_page_and_no_other() {
    let page = read(PAGE);
    let listed: Vec<u8> = first_cells(&chapter(&page, "Exit codes"))
        .iter()
        .map(|code| code.parse().expect("an exit code is a number"))
        .collect();
    assert_eq!(
        listed.len(),
        EVERY_EXIT.len(),
        "one row for each way a run ends: {listed:?}"
    );
    for exit in EVERY_EXIT {
        let documented = listed
            .iter()
            .any(|&code| ExitCode::from(code) == ExitCode::from(exit));
        assert!(documented, "{exit:?} is on the page");
    }
}

/// The environment variables of the page are the ones md2html reads.
///
/// # Panics
///
/// When md2html reads a variable the page does not list, or the page lists
/// one md2html does not read.
#[test]
fn every_environment_variable_is_on_the_page_and_no_other() {
    let page = read(PAGE);
    let documented = first_cells(&chapter(&page, "Environment"));
    let read_by_code = read_variables();
    assert!(
        read_by_code.contains("SOURCE_DATE_EPOCH"),
        "the variables are found: {read_by_code:?}"
    );
    assert_eq!(
        documented, read_by_code,
        "the page's variables, then the code's"
    );
}

/// The help shows every option of the page, each with its other
/// spellings on one line, as the page's row gives them.
///
/// # Panics
///
/// When no line of the help holds all the spellings of one of the page's
/// rows (`-o`, `--out`, `--output`), or the page lists no options.
#[test]
fn the_help_shows_every_option_with_its_spellings_on_one_line() {
    let page = read(PAGE);
    let rows = first_cell_rows(&chapter(&page, "Options"));
    assert!(rows.len() > 30, "the page's options are found: {rows:?}");
    for spellings in rows {
        let together = HELP.lines().any(|line| {
            let words = help_words(line);
            spellings
                .iter()
                .all(|spelling| words.contains(spelling.as_str()))
        });
        assert!(together, "the help shows {spellings:?} on one line");
    }
}

/// The help's `--lang` names how many languages have the page's own
/// words, and each of them by its code.
///
/// # Panics
///
/// When the help has no `--lang` entry, or its entry (its line and the
/// lines indented under it) does not give the number [`FAMILY`] or the
/// code of one of the first [`FAMILY`] of [`LANGS`].
#[test]
fn the_help_names_the_languages_with_words() {
    let mut lines = HELP
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("--lang "));
    let first = lines.next().expect("the help has a --lang entry");
    let entry: Vec<&str> = core::iter::once(first)
        .chain(lines.take_while(|line| line.starts_with(&" ".repeat(23))))
        .collect();
    let words: BTreeSet<&str> = entry.iter().flat_map(|line| help_words(line)).collect();
    assert!(
        words.contains(FAMILY.to_string().as_str()),
        "{FAMILY} in {entry:?}"
    );
    for lang in LANGS.iter().take(FAMILY) {
        assert!(words.contains(lang.code), "{} in {entry:?}", lang.code);
    }
}
