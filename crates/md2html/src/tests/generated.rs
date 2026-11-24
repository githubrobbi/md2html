// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Documents no one wrote: made from the edges of the grammar by a seeded
//! generator, and held to what every page must be, whatever its text.
//!
//! The examples elsewhere say what a given text becomes. These say what is
//! true of any text: the page is written to its end, the same text gives
//! the same bytes, the blocks close in the order they open, and nothing in
//! the page runs that md2html did not put there. The seed is fixed, so a
//! failure names a document that fails again; the weekly run of generated
//! documents (`just generated-documents`) renders thousands more through
//! the program itself.

use super::{article, assert_nested, own_glossary_only, page_with, running_code};
use crate::config::Settings;

/// How many documents a run makes.
const DOCUMENTS: usize = 120;

/// The pieces a document is made of: the edges of the grammar, from marks
/// and brackets to raw HTML, maths, diagrams and front matter.
///
/// No `<div>` or `<details>`: a harmless tag passes as the author wrote
/// it, so a closing tag the author did not open closes an element of the
/// page around it, and the nesting below would report the author's markup
/// (docs/TESTING.md, open findings).
const PIECES: [&str; 69] = [
    "word ",
    "two words ",
    "\n",
    "\n\n",
    "    ",
    "\t",
    "*",
    "**",
    "_",
    "__",
    "~~",
    "`",
    "```",
    "[",
    "]",
    "](",
    ")",
    "![",
    "<",
    ">",
    "&",
    "&amp;",
    "&#x41;",
    "&#0;",
    "\\",
    "$",
    "$$",
    "|",
    "> ",
    "- ",
    "1. ",
    "# ",
    "## ",
    "---\n",
    "===\n",
    "é",
    "日本",
    "😀",
    "\u{200b}",
    "\r\n",
    "<script>alert(1)</script>",
    "<img src=x onerror=alert(1)>",
    "<a href=\"javascript:x\">",
    "[l](javascript:alert(1))",
    "<!--",
    "-->",
    "[^1]",
    "[^1]: note\n",
    "[x]: /url \"t\"\n",
    "[x]",
    "> [!NOTE]\n",
    "| a | b |\n|---|---|\n",
    "- [ ] ",
    "- [x] ",
    "{#id}",
    "{width=50%}",
    "https://a.b/c?d=e&f",
    "<https://x.y>",
    "www.a.b",
    "a@b.c",
    "$x^2$",
    "$$\\frac{a}{b}$$",
    "\\sqrt[3]{x}",
    "```rust\nfn f() {}\n```\n",
    "```mermaid\nflowchart LR\n  A[\"<b>x</b>\"] --> B\n```\n",
    "```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n",
    "*[HTML]: x\n",
    "## Glossary\n\n**term**: what it means\n",
    "\n---\ntitle: T\n---\n",
];

/// A generator of numbers that is the same on every machine (`SplitMix64`).
struct Seeded {
    /// Its state.
    state: u64,
}

impl Seeded {
    /// The next number.
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// A number below `bound`.
    ///
    /// # Panics
    ///
    /// When `bound` is zero, or a `usize` does not fit in a `u64`.
    fn below(&mut self, bound: usize) -> usize {
        let wide = u64::try_from(bound).expect("a bound that fits");
        usize::try_from(self.next() % wide).expect("below a usize bound")
    }

    /// A document of up to 80 pieces, some of them repeated.
    ///
    /// # Panics
    ///
    /// Never: every index is drawn below the length it indexes.
    fn document(&mut self) -> String {
        let pieces = 1 + self.below(80);
        let mut text = String::new();
        for _ in 0..pieces {
            let piece = PIECES.get(self.below(PIECES.len())).expect("a piece");
            let times = [1, 1, 1, 2, 3, 20]
                .get(self.below(6))
                .copied()
                .expect("a count");
            text.push_str(&piece.repeat(times));
        }
        text
    }
}

/// Every generated document makes a whole page, the same each time, with
/// its blocks nested and no running code from its text, with and without
/// `--html`.
///
/// # Panics
///
/// When a page of a generated document is not written to its end, differs
/// from the page of a second render, closes a block out of order, or holds
/// an event handler, a script or a `javascript:` URL in its head or
/// article; the message shows the document.
#[test]
fn every_generated_document_makes_a_sound_page() {
    let mut seeded = Seeded { state: 20_261_124 };
    let rich = Settings {
        html: Some(true),
        ..own_glossary_only()
    };
    for _ in 0..DOCUMENTS {
        let text = seeded.document();
        for settings in [&own_glossary_only(), &rich] {
            let html = page_with(&text, settings);
            assert!(html.ends_with("</body></html>"), "not whole: {text:?}");
            assert_eq!(html, page_with(&text, settings), "not the same: {text:?}");
            assert_nested(&article(&html), &text);
            let found = running_code(&html);
            assert!(found.is_none(), "{found:?} from {text:?}");
        }
    }
}
