// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's own glossary: the chapter that defines terms, or the whole
//! document titled Glossary, the forms a definition takes, the column a
//! meaning stands in, and the other spellings that link to a term.

use super::{article, own_glossary_only};
use crate::document::{Shape, render_doc};

/// Only the chapter named Glossary, at the second level, defines terms.
///
/// It defines them by a bold lead in a paragraph or a list item, or by a
/// table whose "Also" column gives other spellings. A heading below it
/// does not end it; one at its level or above does.
///
/// # Panics
///
/// When the page does not count two terms, `hue` and `color` are not
/// linked (`colour` as `color`), a definition in another chapter or after
/// the glossary is taken (`shade`, `moss`, `fern`, `word`), or a meaning's
/// words are taken as spellings.
#[test]
fn only_the_glossary_chapter_defines_terms() {
    let text = "# T\n\nThe colour, the hue, a shade, a fern, some moss, a word that looks.\n\n\
## Other\n\n**shade**: not here\n\n- **moss**: not here\n\n| Term | Meaning |\n|---|---|\n| fern | not here |\n\n\
## Glossary\n\n### Sub\n\n**hue**: a tint\n\n\
| Term | Meaning | Also |\n|---|---|---|\n| color | what it looks like | colour, , tint |\n\n\
# Next\n\n**word**: none\n";
    let page = render_doc(text, None, None, &own_glossary_only(), Shape::Page, None);
    assert_eq!(page.terms, 2, "hue and color");
    let art = article(&page.html);
    assert!(
        art.contains(
            "data-key=\"color\" href=\"#g-color\" data-def=\"what it looks like\">colour</a>"
        ),
        "colour links color"
    );
    assert!(art.contains("data-key=\"hue\""), "hue is a term");
    for not_term in ["shade", "moss", "fern", "word", "looks"] {
        assert!(
            !art.contains(&format!("data-key=\"{not_term}\"")),
            "{not_term} is no term"
        );
    }
}

/// A document titled Glossary is a glossary as a whole.
///
/// Its tables and bold-lead entries are terms wherever they stand, each
/// entry stands where it is written with its term's anchor, and the foot
/// of the page lists none of them a second time.
///
/// # Panics
///
/// When the page does not count its three terms, an entry has not its
/// term's anchor (`g-cluster` on the row, `g-frs` on the paragraph,
/// `g-mft` on the list item), a mention of a term in another's meaning
/// does not link to its entry, a term links to itself in its own row, the
/// foot lists an entry again, or a link to a Markdown page does not lead
/// to its page and keep its anchor.
#[test]
fn a_glossary_document_is_a_glossary_as_a_whole() {
    let text = "# Glossary\n\nKey terms.\n\n---\n\n\
| Term | Definition |\n|------|-----------|\n\
| **Cluster** | The smallest unit of allocation. See [Concepts §1](concepts.md#1--size-vs-size-on-disk). |\n\n\
## More\n\n**FRS**: a record number, one per cluster.\n\n- **MFT** - the table of FRS records.\n";
    let page = render_doc(text, None, None, &own_glossary_only(), Shape::Page, None);
    assert_eq!(page.terms, 3, "cluster, FRS and MFT");
    let art = article(&page.html);
    assert!(
        art.contains("<tr id=\"g-cluster\"><td><strong>Cluster</strong></td>"),
        "{art}"
    );
    assert!(
        art.contains("<p id=\"g-frs\"><strong>FRS</strong>"),
        "{art}"
    );
    assert!(
        art.contains("<li id=\"g-mft\"><strong>MFT</strong>"),
        "{art}"
    );
    assert!(
        art.contains("href=\"#g-cluster\""),
        "FRS's meaning names a cluster"
    );
    assert!(
        art.contains("href=\"concepts.md#1--size-vs-size-on-disk\""),
        "a link of the page's own text keeps its address when the file is not there"
    );
    assert!(!page.html.contains("<dt id=\"g-"), "no copy at the foot");
}

/// A glossary table's meaning is the column headed for it, in any of the
/// languages md2html speaks, wherever it stands; without such a heading
/// it is the second column.
///
/// # Panics
///
/// When a term does not take its meaning from the column headed
/// `Definition`, `Bedeutung` or `意味`, or from the second column when no
/// column is headed for it.
#[test]
fn the_meaning_is_the_column_headed_for_it() {
    for (table, meaning) in [
        (
            "| Term | Also | Definition |\n|---|---|---|\n| hue | tint | a colour |\n",
            "a colour",
        ),
        (
            "| Begriff | Bedeutung |\n|---|---|\n| hue | eine Farbe |\n",
            "eine Farbe",
        ),
        (
            "| 用語 | 別名 | 意味 |\n|---|---|---|\n| hue | tint | 色 |\n",
            "色",
        ),
        (
            "| Term | Notes |\n|---|---|\n| hue | a shade |\n",
            "a shade",
        ),
    ] {
        let text = format!("# T\n\nThe hue.\n\n## Glossary\n\n{table}");
        let page = render_doc(&text, None, None, &own_glossary_only(), Shape::Page, None);
        assert!(
            page.html.contains(&format!(
                "data-key=\"hue\" href=\"#g-hue\" data-def=\"{meaning}\""
            )),
            "{table}"
        );
    }
}
