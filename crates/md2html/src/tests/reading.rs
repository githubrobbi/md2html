// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! How the blocks of a text are read where the examples leave a rule open.
//!
//! Or where md2html reads a text as its author means it
//! (`Reading::Author`): dashes under a sentence, a list a colon announces,
//! a heading's id, the edges of a table, a footnote's lines, maths blocks,
//! tabs, and containers side by side.

use super::{article, fragment, page};

/// Dashes under one short line that reads like a heading underline it;
/// under a sentence, several lines or a long line they are a rule, unless
/// there are only two of them.
///
/// # Panics
///
/// When one of the texts is not read as the heading or the paragraph and
/// rule this test gives for it.
#[test]
fn dashes_under_a_sentence_are_a_rule() {
    let long = "a".repeat(121);
    let longest = "a".repeat(120);
    for (text, heading) in [
        ("Title\n---\n", true),
        ("A sentence.\n--\n", true),
        (longest.as_str(), true),
        ("A sentence.\n---\n", false),
        ("Ends with a colon:\n---\n", false),
        ("Ends with a comma,\n---\n", false),
        ("Ends with a semicolon;\n---\n", false),
        ("Ends in brackets (so)\n---\n", false),
        ("one\ntwo\n---\n", false),
        (long.as_str(), false),
    ] {
        let markdown = if text.contains('\n') {
            String::from(text)
        } else {
            format!("{text}\n---\n")
        };
        let html = fragment(&markdown);
        assert_eq!(html.starts_with("<h2>"), heading, "{text}: {html}");
        assert_eq!(html.ends_with("<hr />\n"), !heading, "{text}: {html}");
    }
}

/// A numbered list right after a line that ends with a colon starts there.
///
/// It does whatever its first number; without the colon it is the
/// paragraph's text, as the specification has it. An item with no text
/// announces nothing.
///
/// # Panics
///
/// When the announced list (also after bold `**Steps:**`) is not an `<ol>`
/// starting at 5, or the unannounced or empty one is.
#[test]
fn a_colon_announces_a_numbered_list() {
    for announced in [
        "Phase B, with the doctor:\n5. A test\n",
        "**Steps:**\n5. A test\n",
    ] {
        let html = fragment(announced);
        assert!(
            html.ends_with("<ol start=\"5\">\n<li>A test</li>\n</ol>\n"),
            "{html}"
        );
    }
    assert_eq!(
        fragment("Phase B\n5. A test\n"),
        "<p>Phase B\n5. A test</p>\n"
    );
    assert_eq!(fragment("Steps:\n5.\n"), "<p>Steps:\n5.</p>\n");
    assert_eq!(
        fragment("Steps:\n- a bullet\n"),
        "<p>Steps:</p>\n<ul>\n<li>a bullet</li>\n</ul>\n"
    );
}

/// `{#id}` after a heading's text, set off by a space, names its anchor;
/// one with no name, a space or a character an id does not take, or with
/// no space before it, is text.
///
/// # Panics
///
/// When `{#my-id}` does not become the heading's id, or one of the others
/// is not kept as text.
#[test]
fn a_heading_names_its_id() {
    let named = article(&page("# T\n\n## Title {#my-id.v2:x}\n"));
    assert!(named.contains("<h2 id=\"my-id.v2:x\">Title<"), "{named}");
    for kept in ["Title{#x}", "Title {#}", "Title {#a b}", "Title {#a!}"] {
        let art = article(&page(&format!("# T\n\n## {kept}\n")));
        assert!(art.contains(&format!(">{kept}<")), "{kept}: {art}");
    }
}

/// A table needs a pipe in its header row, a delimiter row of dashes with
/// as many cells, and neither row indented four columns or more; a single
/// column is a table when its rows have pipes.
///
/// # Panics
///
/// When one of the tables is not read as one, with its alignments, or one
/// of the texts that are no table is.
#[test]
fn what_makes_a_table() {
    for table in ["| a |\n|---|\n", "   | a |\n|---|\n", "| a |\n   |---|\n"] {
        assert!(fragment(table).starts_with("<table>"), "{table}");
    }
    let aligned = fragment("| a | b | c | d |\n|:-|-:|:-:|---|\n");
    assert!(
        aligned.contains(
            "<th align=\"left\">a</th>\n<th align=\"right\">b</th>\n<th align=\"center\">c</th>\n<th>d</th>"
        ),
        "{aligned}"
    );
    for not_table in [
        "abc\n|---|\n",
        "| a |\n    |---|\n",
        "| x |\n| a |\n",
        "| x |\n| :: |\n",
        "| x |\n| : |\n",
    ] {
        assert!(!fragment(not_table).contains("<table>"), "{not_table}");
    }
}

/// A footnote's text runs on over lazy lines and indented paragraphs; a
/// blank line and then an unindented line end it, and so does another
/// footnote or a block that interrupts a paragraph.
///
/// # Panics
///
/// When the note does not hold its lazy line and its indented paragraph,
/// or holds the text after it, the next note or the quote.
#[test]
fn a_footnote_runs_to_where_it_ends() {
    let html = fragment("Note[^1].\n\n[^1]: first\nlazy line\n\n    indented para\n\nafter\n");
    assert!(
        html.contains("<p>Note<sup class=\"fn\"><a href=\"#fn-1\">1</a></sup>.</p>\n<p>after</p>"),
        "{html}"
    );
    assert!(
        html.contains("<li id=\"fn-1\">\n<p>first\nlazy line</p>\n<p>indented para</p>\n</li>"),
        "{html}"
    );
    let two = fragment("Note[^1][^2].\n\n[^1]: first\n[^2]: second\n");
    assert!(
        two.contains("<p>first</p>\n</li>\n<li id=\"fn-2\">"),
        "{two}"
    );
    let quoted = fragment("Note[^1].\n\n[^1]: first\n> quote\n");
    assert!(quoted.contains("<blockquote>\n<p>quote</p>"), "{quoted}");
}

/// `$$` opens a maths block, on one line or over several; a pair of
/// `$$` with nothing between is text.
///
/// # Panics
///
/// When the one-line or the fenced formula is not a block, or the empty
/// one is not a paragraph.
#[test]
fn a_maths_block_holds_a_formula() {
    assert!(fragment("$$ x^2 $$\n").starts_with("<math display=\"block\">"));
    assert!(fragment("$$\nx\n$$\n").starts_with("<math display=\"block\">"));
    assert_eq!(fragment("$$  $$\n"), "<p>$$  $$</p>\n");
}

/// A tab in the indentation reaches to the next stop of four, wherever it
/// starts; an item's text after a marker and a tab starts at that stop.
///
/// # Panics
///
/// When the code after a space and a tab, or after a quote's marker and a
/// tab, keeps columns that belong to the indentation.
#[test]
fn a_tab_reaches_the_next_stop() {
    assert_eq!(fragment(" \tcode\n"), "<pre><code>code\n</code></pre>\n");
    assert_eq!(
        fragment(">\t\tcode\n"),
        "<blockquote>\n<pre><code>  code\n</code></pre>\n</blockquote>\n"
    );
    assert_eq!(
        fragment("-\t\tcode\n"),
        "<ul>\n<li>\n<pre><code>  code\n</code></pre>\n</li>\n</ul>\n"
    );
}

/// Containers side by side are each as deep as the first: seventy quotes
/// one after another are seventy quotes, not text after the sixty-fourth.
///
/// # Panics
///
/// When one of the seventy quotes is not a quote.
#[test]
fn containers_side_by_side_do_not_add_up() {
    let html = fragment(&"> a\n\n".repeat(70));
    assert_eq!(html.matches("<blockquote>").count(), 70, "{html}");
}

/// A fenced code block inside a list item keeps what looks like a list
/// marker or a blank line inside it as code.
///
/// # Panics
///
/// When the dash or the line after the blank inside the fence leaves the
/// code block.
#[test]
fn a_fence_in_an_item_holds_its_lines() {
    assert_eq!(
        fragment("- a\n\n  ```\n  - not\n\n  item\n  ```\n"),
        "<ul>\n<li>\n<p>a</p>\n<pre><code>- not\n\nitem\n</code></pre>\n</li>\n</ul>\n"
    );
}

/// A code block not closed by the end of the text ends with its last
/// line; the text's final line ending starts no empty line of code.
///
/// # Panics
///
/// When the unclosed fence holds an empty line after its last.
#[test]
fn an_unclosed_fence_ends_with_the_text() {
    assert_eq!(fragment("```\naaa\n"), "<pre><code>aaa\n</code></pre>\n");
    assert_eq!(fragment("```\n"), "<pre><code></code></pre>\n");
}
