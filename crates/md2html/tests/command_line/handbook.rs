// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A handbook of several pages written for GitHub: its glossary file
//! lends its terms to every page.

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
