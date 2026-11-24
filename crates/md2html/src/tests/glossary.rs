// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's own glossary: the chapter that defines terms, the forms a
//! definition takes, and the other spellings that link to a term.

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
