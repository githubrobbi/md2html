// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Where the pages go, and what each stream carries while they are made.

use super::{Folder, NOTES};

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
