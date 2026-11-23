// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the block parser makes of small documents.

use super::{normalize_label, parse};
use crate::model::{Block, Task};

/// The kinds of the top-level blocks of `text`, maths on.
fn kinds(text: &str) -> Vec<&'static str> {
    parse(text, 1, true, false)
        .0
        .iter()
        .map(|block| match block {
            Block::Heading(_) => "h",
            Block::Paragraph(_) => "p",
            Block::Code(_) => "code",
            Block::Html(_) => "html",
            Block::Rule => "hr",
            Block::Quote(_) => "quote",
            Block::List(_) => "list",
            Block::Table(_) => "table",
            Block::Math(_) => "math",
        })
        .collect()
}

/// A list item indented under another becomes a list inside that item.
///
/// # Panics
///
/// Panics when `- a` with ` - b` and ` - c` under it does not give one tight
/// list of two items whose first holds a list of two, for instance when the
/// indented items are read as items of the outer list.
#[test]
fn a_nested_list_nests() {
    let (blocks, _defs) = parse("- a\n  - b\n  - c\n- d\n", 1, false, false);
    let Some(Block::List(list)) = blocks.first() else {
        panic!("a list");
    };
    assert!(list.tight);
    let [first, _second] = list.items.as_slice() else {
        panic!("two items");
    };
    assert!(matches!(first.blocks.get(1), Some(Block::List(inner)) if inner.items.len() == 2));
}

/// A table may start on the line after a paragraph, with no blank line between.
///
/// # Panics
///
/// Panics when `Text:` followed by a table's rows does not give a paragraph and
/// then a table, for instance when the rows are read as more of the paragraph.
#[test]
fn a_table_may_follow_a_line_of_text() {
    assert_eq!(kinds("Text:\n| a | b |\n|---|---|\n| 1 | 2 |\n"), [
        "p", "table"
    ]);
}

/// Task markers become the items' state, and a footnote definition is recorded.
///
/// # Panics
///
/// Panics when `- [x] done` is not a done task or `- [ ] open` an open one,
/// when the two items are not one list, or when `[^1]: The note.` is not kept
/// among the footnotes.
#[test]
fn task_items_and_footnotes() {
    let (blocks, defs) = parse(
        "- [x] done\n- [ ] open\n\n[^1]: The note.\n",
        1,
        false,
        false,
    );
    let Some(Block::List(list)) = blocks.first() else {
        panic!("a list");
    };
    let [done, open] = list.items.as_slice() else {
        panic!("two items");
    };
    assert_eq!(done.task, Some(Task::Done));
    assert_eq!(open.task, Some(Task::Open));
    assert!(defs.notes.contains_key("1"));
}

/// An HTML comment over several lines is one block of HTML.
///
/// # Panics
///
/// Panics when a comment from `<!--` to `-->` on later lines is not read as one
/// HTML block followed by the paragraph after it, for instance when it ends at
/// the first blank-free line.
#[test]
fn a_comment_spanning_lines_is_one_block() {
    assert_eq!(kinds("<!--\nhidden\n-->\n\nText.\n"), ["html", "p"]);
}

/// Both ways of writing a maths block are read as maths.
///
/// # Panics
///
/// Panics when a `$$` block or a fenced block of language `math` is read as
/// anything but a maths block.
#[test]
fn maths_blocks() {
    assert_eq!(kinds("$$\nx^2\n$$\n\n```math\ny\n```\n"), ["math", "math"]);
}

/// A footnote label obeys the 999-character limit of a link label.
///
/// # Panics
///
/// Panics when a footnote whose label is 999 characters is not recorded, when
/// one whose label is 1,000 characters is, or when the refused line does not
/// stay as one paragraph.
#[test]
fn a_footnote_label_longer_than_a_link_label_defines_nothing() {
    let fits = "n".repeat(999);
    let too_long = "n".repeat(1000);
    let (_, kept) = parse(&format!("[^{fits}]: kept\n"), 1, true, false);
    assert!(kept.notes.contains_key(&normalize_label(&fits)));
    let (blocks, refused) = parse(&format!("[^{too_long}]: text\n"), 1, true, false);
    assert!(refused.notes.is_empty());
    assert_eq!(blocks.len(), 1, "the line stays a paragraph");
}

/// A tab after a list marker counts to its next stop of four.
///
/// # Panics
///
/// Panics when `-\tfoo` is not a list (the tab read as no space after the
/// marker), when `-\t\tfoo` is not an item holding the code `  foo`, or when a
/// fenced line `-\tx` loses its tab.
#[test]
fn a_tab_after_a_list_marker_counts_to_its_stop() {
    assert_eq!(kinds("-\tfoo\n"), ["list"]);
    let (blocks, _) = parse("-\t\tfoo\n\n```\n-\tx\n```\n", 1, false, false);
    let [Block::List(list), Block::Code(fenced)] = blocks.as_slice() else {
        panic!("a list and a code block");
    };
    let held = list.items.first().and_then(|item| item.blocks.first());
    assert!(matches!(held, Some(Block::Code(code)) if code.text == "  foo\n"));
    assert_eq!(fenced.text, "-\tx\n");
}

/// A lazy line is the text of the paragraph it continues, whatever it looks
/// like.
///
/// # Panics
///
/// Panics when a line under a quote's paragraph, without the `>`, is read as
/// a setext underline, a list, a quote or a table's delimiter row instead of
/// more of the paragraph.
#[test]
fn a_lazy_line_is_paragraph_text() {
    for text in [
        "> a\nb\n===\n",
        "> a\n    - b\n",
        "> a\n    > b\n",
        "> a\n|x|\n|-|\n",
    ] {
        let (blocks, _) = parse(text, 1, false, true);
        let [Block::Quote(quote)] = blocks.as_slice() else {
            panic!("one quote: {text:?}");
        };
        assert!(
            matches!(quote.blocks.as_slice(), [Block::Paragraph(_)]),
            "{text:?}"
        );
    }
}

/// A link reference definition's label and title may span lines, and what
/// is under nothing but definitions underlines nothing.
///
/// # Panics
///
/// Panics when a label over two lines or a title over two lines is not
/// read as one definition, when `--` under a definition is not a paragraph
/// of its own, or when `---` there is not a rule.
#[test]
fn definitions_span_lines_and_underline_nothing() {
    let (blocks, defs) = parse("[a\nB]:\n/u 'x\ny'\n", 1, false, true);
    assert!(blocks.is_empty());
    let reference = defs.links.get("a b");
    assert!(reference.is_some_and(|it| it.href == "/u" && it.title.as_deref() == Some("x\ny")));
    assert_eq!(kinds("[a]: /u\n--\n"), ["p"]);
    assert_eq!(kinds("[a]: /u\n---\n"), ["hr"]);
}
