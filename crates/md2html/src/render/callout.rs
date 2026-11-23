// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Callouts: a quote whose first words name a kind (`[!NOTE]`,
//! `**Warning:**`, `Tip:`) is a note, a tip, a warning set apart.
//!
//! The words an author may open one with are read here, where the quote
//! is read: the interface words of every language (`config`), and the
//! English and German spellings authors use beside them.

use super::Renderer;
use crate::model::{Block, Callout, Paragraph};
use crate::scan::Scan;
use crate::text::esc;

/// The opening of a callout: its kind, and the text after the words that
/// named it.
#[derive(Debug, PartialEq, Eq)]
struct Opening<'text> {
    /// The kind it names.
    kind: Callout,
    /// The text after the naming words, from its first non-space.
    rest: &'text str,
}

impl Renderer<'_, '_> {
    /// A quote as a callout, if its first paragraph opens with a kind.
    pub(super) fn callout(&mut self, blocks: &[Block]) -> Option<String> {
        let (Block::Paragraph(first), later) = blocks.split_first()? else {
            return None;
        };
        let opening = opening(&first.text)?;
        let mut inner = String::new();
        if !opening.rest.trim().is_empty() {
            let rest = Block::Paragraph(Paragraph {
                text: opening.rest.to_owned(),
                line: first.line,
            });
            inner.push_str(&self.block(&rest, false));
        }
        for block in later {
            inner.push_str(&self.block(block, false));
        }
        Some(format!(
            "<div class=\"callout {}\" role=\"note\"><span class=\"mark\">{}</span><div class=\"body\">{inner}</div></div>",
            opening.kind.class(),
            esc(self.inp.ui.callouts.word(opening.kind))
        ))
    }
}

/// The English and German words beside the interface's own that open a
/// callout of `kind`, lowercased.
const fn spellings(kind: Callout) -> &'static [&'static str] {
    match kind {
        Callout::Note => &["note", "hinweis", "info", "anmerkung"],
        Callout::Tip => &["tip", "tipp", "hint"],
        Callout::Important => &["important", "wichtig"],
        Callout::Warning => &["warning", "achtung", "warnung"],
        Callout::Caution => &["caution", "vorsicht", "danger", "gefahr"],
    }
}

/// The kind a word names, in any case: one of the spellings above, or a
/// callout's name in any language md2html speaks.
fn named(word: &str) -> Option<Callout> {
    let lower = word.to_lowercase();
    Callout::ALL
        .into_iter()
        .find(|&kind| spellings(kind).contains(&lower.as_str()))
        .or_else(|| {
            crate::config::all_ui()
                .iter()
                .find_map(|ui| ui.callouts.kind_named(&lower))
        })
}

/// A callout's opening: GitHub's `[!NOTE]`, or a bold or colon-ended word
/// (`**Note:**`, `**Tip.**`, `Warning:`).
fn opening(text: &str) -> Option<Opening<'_>> {
    let start = text.trim_start();
    if let Some(marked) = start.strip_prefix("[!") {
        let (word, rest) = marked.split_once(']')?;
        return Some(Opening {
            kind: named(word)?,
            rest: rest.trim_start(),
        });
    }
    let mut scan = Scan::new(start);
    let bold = scan.eat_str("**");
    let kind = named(scan.take_while(char::is_alphabetic))?;
    if bold {
        scan.eat_if(|ch| matches!(ch, ':' | '.'));
        if !scan.eat_str("**") {
            return None;
        }
        scan.eat_if(|ch| matches!(ch, ':' | '.'));
    } else if !scan.eat(':') {
        return None;
    }
    Some(Opening {
        kind,
        rest: scan.rest().trim_start(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Opening, opening};
    use crate::model::Callout;

    /// The kind `text` opens with.
    fn kind(text: &str) -> Option<Callout> {
        opening(text).map(|found| found.kind)
    }

    #[test]
    fn callouts_are_named_by_their_first_words() {
        assert_eq!(kind("**Note:** read this"), Some(Callout::Note));
        assert_eq!(kind("**Note.** read this"), Some(Callout::Note));
        assert_eq!(kind("[!WARNING]\nCareful"), Some(Callout::Warning));
        assert_eq!(kind("[!CAUTION]"), Some(Callout::Caution));
        assert_eq!(kind("Tip: try it"), Some(Callout::Tip));
        assert_eq!(kind("**Achtung:** heiß"), Some(Callout::Warning));
        assert_eq!(kind("Note that this is a plain quote."), None);
        assert_eq!(kind("Notebooks: plain"), None);
        assert_eq!(kind("**Note** without a stop"), Some(Callout::Note));
        assert_eq!(kind("**Note: unclosed"), None);
        assert_eq!(
            opening("[!NOTE]\nThe text"),
            Some(Opening {
                kind: Callout::Note,
                rest: "The text"
            })
        );
    }
}
