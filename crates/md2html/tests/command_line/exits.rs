// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! How a run ends: each exit code of `docs/command-line.md`, from the
//! situation that gives it, with its message on standard error.

use super::{Folder, NOTES, Ran};

/// `--help` and `--version` print to standard output and end with 0.
///
/// # Panics
///
/// When either ends otherwise, the help has no usage line, or the version
/// is not the crate's.
#[test]
fn help_and_version_end_with_0() {
    let folder = Folder::new("help");
    let help = folder.run(&["--help"]);
    help.ends_with(0);
    assert!(
        help.out.contains("usage: md2html [options] <input>..."),
        "{}",
        help.out
    );
    let version = folder.run(&["-V"]);
    version.ends_with(0);
    assert_eq!(
        version.out,
        format!("md2html {}\n", env!("CARGO_PKG_VERSION"))
    );
}

/// A wrong command line ends with 64 and the usage line.
///
/// The wrong ones: an unknown option, no input, a page over its own
/// Markdown or over Markdown from standard input, several pages to
/// standard output, and `--watch` or `--serve` on standard input.
///
/// # Panics
///
/// When one of them ends otherwise, says nothing on standard error, or
/// writes a page.
#[test]
fn a_wrong_command_line_ends_with_64() {
    let folder = Folder::new("usage");
    folder.write("a.md", NOTES);
    folder.write("b.md", NOTES);
    let unknown = folder.run(&["--no-such-option", "a.md"]);
    unknown.ends_with(64);
    assert!(unknown.err.contains("usage: md2html"), "{}", unknown.err);
    folder.run(&[]).ends_with(64);
    let over = folder.run(&["a.md", "-o", "a.md"]);
    over.ends_with(64);
    assert_eq!(folder.read("a.md"), NOTES, "the Markdown is untouched");
    folder
        .run_with_input(&["-", "-o", "b.md"], "# Over\n")
        .ends_with(64);
    assert_eq!(
        folder.read("b.md"),
        NOTES,
        "standard input writes no page over Markdown"
    );
    folder.run(&["a.md", "b.md", "-o", "-"]).ends_with(64);
    for lasting in ["--watch", "--serve"] {
        let ran = folder.run_with_input(&[lasting, "-"], NOTES);
        ran.ends_with(64);
        assert!(ran.err.contains("needs files, not stdin"), "{}", ran.err);
    }
    assert!(!folder.has("a.html") && !folder.has("b.html"));
}

/// An input that is not there ends the run with 66 before any page is
/// made; one that is not UTF-8, or is larger than the limit, ends it with
/// 66 after the other inputs are made.
///
/// # Panics
///
/// When one of them ends otherwise, the message does not name the input
/// (and, for the size, how to raise the limit), a page is made beside a
/// missing input, or the good input beside an unreadable one is not made.
#[test]
fn an_input_that_cannot_be_read_ends_with_66() {
    let folder = Folder::new("noinput");
    folder.write("good.md", NOTES);
    folder.write("latin1.md", b"caf\xe9\n");
    folder.write("large.md", "a".repeat(1_100_000));
    let missing = folder.run(&["missing.md", "good.md"]);
    missing.ends_with(66);
    assert_eq!(missing.err, "md2html: missing.md: no such file\n");
    assert!(!folder.has("good.html"), "nothing is made");
    let latin1 = folder.run(&["latin1.md", "good.md"]);
    latin1.ends_with(66);
    assert!(
        latin1.err.starts_with("md2html: latin1.md: "),
        "{}",
        latin1.err
    );
    assert!(folder.has("good.html"), "the run goes on");
    let large = folder.run(&["--max-input", "1", "large.md"]);
    large.ends_with(66);
    assert_eq!(
        large.err,
        "md2html: large.md: larger than 1 MB (--max-input raises the limit)\n"
    );
}

/// A page that cannot be written ends with 73: its folder would be under
/// a file.
///
/// # Panics
///
/// When the run ends otherwise or does not name the page.
#[test]
fn a_page_that_cannot_be_written_ends_with_73() {
    let folder = Folder::new("cantcreate");
    folder.write("a.md", NOTES);
    folder.write("file", "a file, not a folder");
    let ran = folder.run(&["a.md", "-o", "file/a.html"]);
    ran.ends_with(73);
    assert!(ran.err.contains("file"), "{}", ran.err);
}

/// When standard output cannot be written, a run that made its pages ends
/// with 74 and says so on standard error; a page meant for standard output
/// is not made, and that ends with 73.
///
/// # Panics
///
/// When the runs end otherwise, or do not say why on standard error.
#[test]
fn a_closed_standard_output_ends_with_74_or_73() {
    let folder = Folder::new("closed");
    folder.write("a.md", NOTES);
    let summary = folder.run_with_stdout_closed(&["a.md"]);
    summary.ends_with(74);
    assert!(folder.has("a.html"), "the page is made");
    assert!(
        summary.err.contains("standard output could not be written"),
        "{}",
        summary.err
    );
    let page = folder.run_with_stdout_closed(&["a.md", "-o", "-"]);
    page.ends_with(73);
    assert!(
        page.err.contains("writing the page to standard output"),
        "{}",
        page.err
    );
}

/// `--pdf` with an `MD2HTML_CHROME` that names no file ends with 70 and
/// says so; the page itself is still written.
///
/// # Panics
///
/// When the run ends otherwise, the page is missing, or the message does
/// not name the PDF.
#[test]
fn a_pdf_without_chrome_ends_with_70() {
    let folder = Folder::new("nochrome");
    folder.write("a.md", NOTES);
    let ran = Ran::of(
        folder
            .md2html(&["--pdf", "a.md"])
            .env("MD2HTML_CHROME", folder.path().join("no-chrome-here")),
    );
    ran.ends_with(70);
    assert!(folder.has("a.html") && !folder.has("a.pdf"));
    assert!(ran.err.starts_with("md2html: a.pdf: "), "{}", ran.err);
}
