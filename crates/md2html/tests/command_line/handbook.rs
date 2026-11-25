// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A handbook of several pages written for GitHub: its glossary file
//! lends its terms to every page.

use std::path::Path;

use super::{Folder, Ran};

/// A handbook's glossary, a page of its own as GitHub shows one.
///
/// `# Glossary`, then a table whose meanings link to the handbook's pages
/// by the anchors GitHub gives their headings.
const GLOSSARY: &str = "# Glossary\n\nKey terms.\n\n---\n\n| Term | Definition |\n|------|-----------|\n\
| **Cluster** | The smallest unit of allocation. See [Concepts §1](concepts.md#1--size-vs-size-on-disk). |\n\
| **FRS** | A record number, one per cluster. |\n";

/// A page of the handbook that uses the glossary's terms and has the
/// heading the glossary links to.
const CONCEPTS: &str = "# Concepts\n\nEvery file takes a cluster at least.\n\n## 1 · Size vs Size on Disk\n\nThe FRS names it.\n";

/// The glossary file of `MD2HTML_GLOSSARY` lends a handbook's pages its
/// terms.
///
/// Each is linked to its meaning; the glossary's own page shows its table
/// with an anchor on each term's row, and lists no copy of them; a
/// meaning's link written for GitHub leads to the page and its heading.
///
/// # Panics
///
/// When a run does not end with 0, the page does not count both terms as
/// shared or does not link `cluster` to its meaning, the glossary's page
/// counts a term as shared, has not its rows at the terms' anchors or
/// lists them again at its foot, or a meaning's link does not lead to
/// `concepts.html#1--size-vs-size-on-disk`, where the heading is.
#[test]
fn a_glossary_file_lends_a_handbook_its_terms() {
    let folder = Folder::new("handbook-glossary");
    folder.write("manual/glossary.md", GLOSSARY);
    folder.write("manual/concepts.md", CONCEPTS);
    let ran = Ran::of(
        folder
            .md2html(&["manual", "-o", "site/"])
            .env("MD2HTML_GLOSSARY", folder.path().join("manual/glossary.md")),
    );
    ran.ends_with(0);
    assert!(
        ran.out.contains("concepts.html (")
            && ran
                .out
                .contains(" bytes, 2 glossary terms, 2 of them shared)"),
        "{}",
        ran.out
    );
    assert!(
        ran.out
            .contains(" bytes, 2 glossary terms, 0 of them shared)"),
        "{}",
        ran.out
    );
    let concepts = folder.read("site/concepts.html");
    assert!(
        concepts.contains("data-key=\"cluster\" href=\"#g-cluster\" data-def=\"The smallest unit of allocation. See Concepts §1.\">cluster</a>"),
        "cluster links its shared meaning"
    );
    assert!(
        concepts.contains("<h2 id=\"1--size-vs-size-on-disk\">"),
        "the heading has GitHub's anchor"
    );
    assert!(
        concepts.contains("href=\"concepts.html#1--size-vs-size-on-disk\""),
        "the meaning at the foot leads to the page, not its Markdown"
    );
    let glossary = folder.read("site/glossary.html");
    assert!(glossary.contains("<tr id=\"g-cluster\">") && glossary.contains("<tr id=\"g-frs\">"));
    assert!(!glossary.contains("<dt id=\"g-"), "no copy at the foot");
    assert!(
        glossary.contains("<a href=\"concepts.html#1--size-vs-size-on-disk\">Concepts §1</a>"),
        "the row's link leads to the page's heading"
    );
}

/// A page of the book that links to its other page's headings, and to a
/// page outside the run.
const FIRST: &str = "# First\n\n## One\n\nSee [the setup](second.md#setup), [nothing](second.md#nope), \
[this](first.md#one), [not this](first.md#missing) and [elsewhere](../elsewhere/third.md#nope).\n";

/// The other page of the book.
const SECOND: &str = "# Second\n\n## Setup\n\nDone.\n";

/// A page beside the book, never an input of the run.
const THIRD: &str = "# Third\n\nNo headings.\n";

/// `--check` over a folder finds a link to another page of the run whose
/// anchor that page does not have, at the link's line.
///
/// A page outside the run is not looked into, and a page given alone has
/// no other page to look into.
///
/// # Panics
///
/// When the run over the folder does not end with 1, does not name the
/// two broken links at line 5 in the existing wording, names a good link,
/// or names the page outside the run; or when the run over the first page
/// alone does not end with 0 with nothing said of another page.
#[test]
fn check_finds_a_link_to_another_pages_missing_heading() {
    let folder = Folder::new("handbook-check");
    folder.write("book/first.md", FIRST);
    folder.write("book/second.md", SECOND);
    folder.write("elsewhere/third.md", THIRD);
    let ran = folder.run(&["--check", "book"]);
    ran.ends_with(1);
    let first = Path::new("book").join("first.md");
    let told: Vec<&str> = ran.err.lines().collect();
    assert_eq!(
        told,
        [
            format!(
                "{}:5: a link points at second.md#nope, which is nowhere on second.md",
                first.display()
            ),
            format!(
                "{}:5: a link points at first.md#missing, which is nowhere on first.md",
                first.display()
            ),
            String::from("md2html: 2 to look at"),
        ],
        "{}",
        ran.err
    );
    let alone = folder.run(&["--check", "book/first.md"]);
    alone.ends_with(1);
    assert!(
        !alone.err.contains("second.md#") && alone.err.contains("first.md#missing"),
        "{}",
        alone.err
    );
}
