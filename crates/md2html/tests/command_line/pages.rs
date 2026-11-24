// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Where the pages go, and what each stream carries while they are made.

use std::fs::File;
use std::process::Stdio;

use super::{Folder, NOTES, Ran};

/// A page is written beside its Markdown, and standard output says so in
/// one line; standard error stays empty.
///
/// # Panics
///
/// When the run does not end with 0, `notes.html` is not a whole page, the
/// summary is not one line naming both files, or anything goes to
/// standard error.
#[test]
fn a_page_is_written_beside_its_markdown() {
    let folder = Folder::new("beside");
    folder.write("notes.md", NOTES);
    let ran = folder.run(&["notes.md"]);
    ran.ends_with(0);
    let page = folder.read("notes.html");
    assert!(page.starts_with("<!doctype html>"), "a page");
    assert!(page.ends_with("</body></html>"), "the page is whole");
    assert!(page.contains("<h1>Notes</h1>"));
    assert_eq!(ran.out.lines().count(), 1, "{}", ran.out);
    assert!(
        ran.out.starts_with("md2html: notes.md -> notes.html ("),
        "{}",
        ran.out
    );
    assert_eq!(ran.err, "");
}

/// The older form `md2html <in.md> <out.html>` names the page, and so does
/// `-o`; `-q` leaves the summary out.
///
/// # Panics
///
/// When either form does not write the page it names, or `-q` prints
/// anything.
#[test]
fn the_page_can_be_named() {
    let folder = Folder::new("named");
    folder.write("notes.md", NOTES);
    folder.run(&["notes.md", "old-form.html"]).ends_with(0);
    assert!(folder.has("old-form.html"));
    let quiet = folder.run(&["-q", "notes.md", "-o", "sub/named.html"]);
    quiet.ends_with(0);
    assert!(
        folder.has("sub/named.html"),
        "the folder on the way is made"
    );
    assert_eq!((quiet.out.as_str(), quiet.err.as_str()), ("", ""));
}

/// The page of standard input goes to standard output, and nothing else
/// does.
///
/// # Panics
///
/// When the run does not end with 0, standard output is not the whole
/// page, or anything goes to standard error.
#[test]
fn standard_input_becomes_standard_output() {
    let folder = Folder::new("stdin");
    let ran = folder.run_with_input(&["-"], NOTES);
    ran.ends_with(0);
    assert!(ran.out.starts_with("<!doctype html>"), "a page");
    assert!(ran.out.ends_with("</body></html>"), "the page is whole");
    assert!(ran.out.contains("<h1>Notes</h1>"));
    assert_eq!(ran.err, "");
}

/// A page sent to standard output is the same bytes whether standard
/// output is a pipe or a file, and the null device takes a page or a
/// summary without a failure.
///
/// # Panics
///
/// When a run does not end with 0, says anything on standard error, or
/// the page in a file differs from the page through a pipe.
#[test]
fn standard_output_takes_a_pipe_a_file_or_the_null_device() {
    let folder = Folder::new("outputs");
    folder.write("a.md", NOTES);
    let piped = folder.run(&["a.md", "-o", "-"]);
    piped.ends_with(0);
    assert!(piped.out.ends_with("</body></html>"), "the page is whole");
    let file = File::create(folder.path().join("sent.html")).expect("a file for the page");
    let sent = folder.run_with_stdout(&["a.md", "-o", "-"], Stdio::from(file));
    sent.ends_with(0);
    assert_eq!(sent.err, "");
    assert_eq!(folder.read("sent.html"), piped.out, "into a file");
    for args in [["a.md", "-o", "-"], ["a.md", "-o", "nulled.html"]] {
        let nulled = folder.run_with_stdout(&args, Stdio::null());
        nulled.ends_with(0);
        assert_eq!(nulled.err, "", "{args:?}");
    }
}

/// `--fragment` writes only the Markdown's HTML.
///
/// # Panics
///
/// When the fragment of a heading and a paragraph is anything else.
#[test]
fn a_fragment_is_the_markdown_alone() {
    let folder = Folder::new("fragment");
    let ran = folder.run_with_input(&["--fragment", "-"], "# Notes\n\nA *word*.\n");
    ran.ends_with(0);
    assert_eq!(ran.out, "<h1>Notes</h1>\n<p>A <em>word</em>.</p>\n");
}

/// `--check` writes nothing, says what to fix as `<input>: <what>`, and
/// ends with 1 when there is anything; with nothing to fix it says so and
/// ends with 0. With `-q` only the findings are said.
///
/// # Panics
///
/// When a page is written, the link to nowhere is not reported in that
/// form with the count after it, the exit codes are not 1 and 0, or `-q`
/// keeps the count or loses the finding.
#[test]
fn check_reports_and_writes_nothing() {
    let folder = Folder::new("check");
    folder.write("broken.md", "# A\n\nSee [x](#nowhere).\n");
    folder.write("fine.md", NOTES);
    let found = folder.run(&["--check", "broken.md"]);
    found.ends_with(1);
    assert_eq!(
        found.err,
        "broken.md: a link points at #nowhere, which is nowhere on the page\nmd2html: 1 to look at\n"
    );
    assert_eq!(found.out, "");
    let quiet = folder.run(&["--check", "-q", "broken.md"]);
    quiet.ends_with(1);
    assert_eq!(
        quiet.err,
        "broken.md: a link points at #nowhere, which is nowhere on the page\n"
    );
    let clean = folder.run(&["--check", "fine.md"]);
    clean.ends_with(0);
    assert_eq!(clean.out, "md2html: nothing to fix\n");
    assert!(!folder.has("broken.html") && !folder.has("fine.html"));
}

/// Pages rendered into a folder keep their names, and `--nav` links each
/// to the one before and after it.
///
/// # Panics
///
/// When a page is missing from `out/`, the first does not link the second
/// as next, the second the first as previous, or the summary does not have
/// a line for each.
#[test]
fn pages_of_one_run_link_each_other() {
    let folder = Folder::new("nav");
    folder.write("a.md", "# A\n\nFirst.\n");
    folder.write("b.md", "# B\n\nSecond.\n");
    let ran = folder.run(&["a.md", "b.md", "-o", "out/", "--nav"]);
    ran.ends_with(0);
    assert!(
        folder
            .read("out/a.html")
            .contains("rel=\"next\" href=\"b.html\"")
    );
    assert!(
        folder
            .read("out/b.html")
            .contains("rel=\"prev\" href=\"a.html\"")
    );
    assert_eq!(ran.out.lines().count(), 2, "{}", ran.out);
}

/// A folder means every Markdown file under it, but not the hidden ones.
///
/// # Panics
///
/// When a page is not written beside each Markdown file of the folder and
/// its subfolder, or one is written for the hidden file.
#[test]
fn a_folder_is_every_markdown_file_in_it() {
    let folder = Folder::new("folder");
    folder.write("docs/index.md", NOTES);
    folder.write("docs/part/one.md", NOTES);
    folder.write("docs/.hidden.md", NOTES);
    folder.write("docs/notes.txt", "not Markdown");
    folder.run(&["docs"]).ends_with(0);
    assert!(folder.has("docs/index.html") && folder.has("docs/part/one.html"));
    assert!(!folder.has("docs/.hidden.html") && !folder.has("docs/notes.html"));
}

/// `--epub` writes a book beside the page and names it.
///
/// # Panics
///
/// When the book is not a ZIP archive beside the page, or the summary does
/// not name it.
#[test]
fn a_book_is_written_beside_the_page() {
    let folder = Folder::new("epub");
    folder.write("notes.md", NOTES);
    let ran = folder.run(&["--epub", "notes.md"]);
    ran.ends_with(0);
    let book = std::fs::read(folder.path().join("notes.epub")).expect("the book");
    assert!(book.starts_with(b"PK"), "a ZIP archive");
    assert!(ran.out.starts_with("md2html: notes.epub ("), "{}", ran.out);
}

/// The shared glossary of `MD2HTML_GLOSSARY` lends a page the terms it
/// uses and does not define itself; the shared page itself, rendered,
/// is a page of its own terms and no copy of them.
///
/// # Panics
///
/// When the page does not count its own term and the shared one it uses,
/// one of them shared, does not link `fern` to the shared meaning and
/// `moss` to its own, or the shared page counts any term as shared or
/// differs from its page without a shared glossary.
#[test]
fn a_shared_glossary_lends_its_terms() {
    let folder = Folder::new("shared");
    folder.write(
        "glossary.md",
        "# Shared\n\n## Glossary\n\n**fern**: a plant\n\n**moss**: soft\n",
    );
    folder.write(
        "page.md",
        "# Page\n\nA fern on moss.\n\n## Glossary\n\n**moss**: our own\n",
    );
    let shared = folder.path().join("glossary.md");
    let ran = Ran::of(
        folder
            .md2html(&["page.md", "glossary.md"])
            .env("MD2HTML_GLOSSARY", &shared),
    );
    ran.ends_with(0);
    assert!(
        ran.out.contains("page.md -> page.html (")
            && ran
                .out
                .contains(" bytes, 2 glossary terms, 1 of them shared)"),
        "{}",
        ran.out
    );
    assert!(
        ran.out
            .contains(" bytes, 2 glossary terms, 0 of them shared)\n"),
        "{}",
        ran.out
    );
    let page = folder.read("page.html");
    assert!(
        page.contains("data-key=\"fern\" href=\"#g-fern\" data-def=\"a plant\""),
        "fern from the shared glossary"
    );
    assert!(
        page.contains("data-def=\"our own\">moss</a>"),
        "moss, the page's own"
    );
    let with_shared = folder.read("glossary.html");
    folder.run(&["glossary.md"]).ends_with(0);
    assert!(
        with_shared == folder.read("glossary.html"),
        "the shared page is the same page without a shared glossary"
    );
}
