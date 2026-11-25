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

/// A handbook as the README chapter recommends it.
///
/// A start page that links its chapters and a part, a glossary page, a
/// page no link places, one that says it is meant to be so, and a folder
/// no link enters.
fn manual(folder: &Folder) {
    folder.write(
        "manual/README.md",
        "# The Manual\n\n1. [Install](install.md)\n2. [Part](part/README.md)\n3. [Glossary](glossary.md)\n",
    );
    folder.write(
        "manual/install.md",
        "# Install\n\nEvery file takes a cluster.\n",
    );
    folder.write("manual/part/README.md", "# Part\n\n[Step](step.md)\n");
    folder.write("manual/part/step.md", "# Step\n\nOne cluster more.\n");
    folder.write(
        "manual/glossary.md",
        "# Glossary\n\n| Term | Definition |\n|---|---|\n| **Cluster** | The smallest unit, see [Install](install.md). |\n",
    );
    folder.write("manual/stray.md", "# Stray\n\nNo link places it.\n");
    folder.write("manual/kept.md", "---\norphan: true\n---\n# Kept\n");
    folder.write("manual/dev/plan.md", "# Plan\n\nInternal.\n");
}

/// A folder whose start page links its pages is a handbook with no option.
///
/// Each chapter links the chapters before and after it in the start page's
/// order and shows the contents, itself marked; a page no link places is a
/// chapter after the others and a warning, unless it says it is meant; a
/// folder no link enters is one note and plain pages.
///
/// # Panics
///
/// When a run ends otherwise, a chapter's neighbours or contents are
/// others, the warnings or the note are others, or a page of the folder
/// left out shows the contents.
#[test]
fn a_folder_whose_start_page_links_its_pages_is_a_handbook() {
    let folder = Folder::new("handbook-folder");
    manual(&folder);
    let ran = folder.run(&["manual", "-o", "site/"]);
    ran.ends_with(0);
    let stray = Path::new("manual").join("stray.md");
    let warned: Vec<&str> = ran.err.lines().collect();
    assert_eq!(
        warned,
        [
            format!(
                "md2html: 1 page under {} is not in the handbook (no link from {} enters it)",
                Path::new("manual").join("dev").display(),
                Path::new("manual").join("README.md").display()
            ),
            format!(
                "{}: not linked from README.md; it comes after the pages README.md links (orphan: true says that is meant)",
                stray.display()
            ),
        ],
        "{}",
        ran.err
    );
    let step = folder.read("site/part/step.html");
    assert!(
        step.contains("<a class=\"prev\" rel=\"prev\" href=\"README.html\"><small>&larr; Previous</small>Part</a>")
            && step.contains("<a class=\"next\" rel=\"next\" href=\"../glossary.html\"><small>Next &rarr;</small>Glossary</a>"),
        "the start page's order"
    );
    assert!(
        step.contains(
            "<nav class=\"toc hb\" id=\"toc\" aria-label=\"Outline\"><a class=\"dock\" id=\"dock\" href=\"#top\">&lsaquo; keep the outline open</a>\
<ol class=\"book\"><li><a href=\"../README.html\">The Manual</a></li><li><a href=\"../install.html\">Install</a></li>\
<li><a href=\"README.html\">Part</a></li><li class=\"d1 here\"><a href=\"step.html\" aria-current=\"page\">Step</a></li></ol><ol>"
        ),
        "the contents up to the page, before its outline"
    );
    assert!(
        step.contains("<ol class=\"book\"><li><a href=\"../glossary.html\">Glossary</a></li><li><a href=\"../kept.html\">Kept</a></li><li><a href=\"../stray.html\">Stray</a></li></ol></nav>"),
        "the rest after it"
    );
    let plan = folder.read("site/dev/plan.html");
    assert!(
        !plan.contains("class=\"book\"") && !plan.contains("rel=\"next\""),
        "left out"
    );
    let checked = folder.run(&["--check", "manual"]);
    checked.ends_with(1);
    assert!(
        checked.err.contains("not linked from README.md"),
        "{}",
        checked.err
    );
}

/// `--no-handbook` renders the folder's pages as each is rendered alone:
/// the same bytes.
///
/// # Panics
///
/// When a run ends otherwise than with 0, or a page of the folder run
/// differs from the same page rendered alone.
#[test]
fn no_handbook_renders_the_pages_one_by_one() {
    let folder = Folder::new("handbook-off");
    manual(&folder);
    folder
        .run(&["--no-handbook", "manual", "-o", "folder/"])
        .ends_with(0);
    for page in ["install", "part/step", "README"] {
        let out = format!("alone/{page}.html");
        folder
            .run(&[&format!("manual/{page}.md"), "-o", &out])
            .ends_with(0);
        assert_eq!(
            folder.read(&format!("folder/{page}.html")),
            folder.read(&out),
            "{page}"
        );
    }
}

/// A handbook renders to the same pages twice, and from another folder
/// with the folder named another way.
///
/// # Panics
///
/// When a run ends otherwise than with 0, or a page differs between runs.
#[test]
fn a_handbook_renders_the_same_from_anywhere() {
    let folder = Folder::new("handbook-same");
    manual(&folder);
    folder.run(&["manual", "-o", "one/"]).ends_with(0);
    folder.run(&["manual", "-o", "two/"]).ends_with(0);
    let elsewhere = folder.path().join("manual").join("part");
    Ran::of(
        folder
            .md2html(&["../", "-o", "../../three/"])
            .current_dir(&elsewhere),
    )
    .ends_with(0);
    for page in ["README", "install", "part/step", "glossary", "stray"] {
        let one = folder.read(&format!("one/{page}.html"));
        assert_eq!(
            one,
            folder.read(&format!("two/{page}.html")),
            "{page} twice"
        );
        assert_eq!(
            one,
            folder.read(&format!("three/{page}.html")),
            "{page} from elsewhere"
        );
    }
}
