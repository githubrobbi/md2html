// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a start tag ends without an end tag, as HTML's parser reads it.
//!
//! The rules are WHATWG HTML's ("the rules for parsing tokens in HTML
//! content"), for the elements an article may hold and the ones md2html
//! writes around it.
//!
//! Each rule names the elements a start tag ends and the elements its
//! search does not pass: the parser looks down the open elements for the
//! nearest of either, and ends it only when it is one it ends.

/// Elements whose end tag HTML lets an author leave out: closing one of
/// them for the author mends nothing worth telling.
pub(super) const OPTIONAL_END: &[&str] = &[
    "p", "li", "dt", "dd", "td", "th", "tr", "thead", "tbody", "tfoot", "caption", "colgroup",
    "rp", "rt",
];

/// The roots of drawings and formulas, where `<x/>` closes itself.
pub(super) const FOREIGN: &[&str] = &["svg", "math"];

/// The headings: a heading's start tag ends a heading that is the
/// current element.
pub(super) const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

/// The parts of a table outside its cells.
///
/// What md2html writes there, or any text, the browser moves out before
/// the table (or a later table tag drops), so md2html closes them before
/// writing an element of its own.
pub(super) const TABLE_PARTS: &[&str] = &["table", "thead", "tbody", "tfoot", "tr", "colgroup"];

/// The elements `generate implied end tags` ends, from the current one
/// down: `rt` and `rp` do it when a `ruby` is open.
pub(super) const IMPLIED_ENDS: &[&str] = &["dd", "dt", "li", "p", "rp", "rt"];

/// The start tags that end what [`IMPLIED_ENDS`] holds when a `ruby` is
/// open ([`RUBY`]).
pub(super) const RUBY_TEXT: &[&str] = &["rp", "rt"];

/// The start tags of blocks: each ends an open paragraph, as in HTML.
const BLOCK_STARTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "ul",
];

/// What the search for an open element does not pass in "has an element
/// in scope": cells, captions and tables, and where a drawing or a
/// formula lets HTML in.
const SCOPE: &[&str] = &[
    "caption", "table", "td", "th", "title", "desc", "mi", "mo", "mn", "ms", "mtext",
];

/// HTML's special elements an article may hold, but `address`, `div` and
/// `p`: the search of a list item's or a definition's start tag stops at
/// them.
const SPECIAL: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "caption",
    "center",
    "colgroup",
    "dd",
    "details",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "li",
    "nav",
    "ol",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
    "title",
    "desc",
    "mi",
    "mo",
    "mn",
    "ms",
    "mtext",
];

/// The start tags of a table's parts: they end a table's cell, row,
/// section or caption, and outside a table HTML's parser drops them.
pub(super) const TABLE_STARTS: &[&str] = &[
    "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr",
];

/// What a table's part must stand in: outside all of them, HTML's parser
/// drops its tags.
pub(super) const TABLE_CONTEXT: &[&str] = &["table", "td", "th", "caption"];

/// What a start tag ends that the author may leave open.
pub(super) struct Rule {
    /// The elements it ends: the nearest one open.
    pub(super) ends: &'static [&'static str],
    /// The start tags that end them.
    pub(super) by: &'static [&'static str],
    /// The elements the search does not pass: one of [`Self::ends`]
    /// opened before the nearest of these stays open.
    pub(super) within: &'static [&'static str],
}

/// A block ends an open paragraph: "close a p element" for a `p` in
/// button scope.
pub(super) const PARAGRAPH: Rule = Rule {
    ends: &["p"],
    by: BLOCK_STARTS,
    within: SCOPE,
};

/// A `ruby` open in scope lets `rt` and `rp` end what is implied
/// ([`IMPLIED_ENDS`]).
pub(super) const RUBY: Rule = Rule {
    ends: &["ruby"],
    by: RUBY_TEXT,
    within: SCOPE,
};

/// Everything a start tag may end, as HTML's parser ends it, but a heading
/// (which ends only the current element) and ruby text.
pub(super) const RULES: [Rule; 8] = [
    PARAGRAPH,
    // A list item ends the nearest list item, up to a special element.
    Rule {
        ends: &["li"],
        by: &["li"],
        within: SPECIAL,
    },
    // A definition's term or text ends the nearest of either.
    Rule {
        ends: &["dd", "dt"],
        by: &["dd", "dt"],
        within: SPECIAL,
    },
    // In a cell, a table's tag ends the cell.
    Rule {
        ends: &["td", "th"],
        by: TABLE_STARTS,
        within: &["table"],
    },
    // In a row, a row's or a section's tag ends the row.
    Rule {
        ends: &["tr"],
        by: &[
            "caption", "col", "colgroup", "tbody", "tfoot", "thead", "tr",
        ],
        within: &["table"],
    },
    // In a section, another section's tag ends it.
    Rule {
        ends: &["thead", "tbody", "tfoot"],
        by: &["caption", "col", "colgroup", "tbody", "tfoot", "thead"],
        within: &["table"],
    },
    // In a caption, a table's tag ends the caption.
    Rule {
        ends: &["caption"],
        by: TABLE_STARTS,
        within: &["table"],
    },
    // A link ends the link it is in, up to a cell or a caption (the
    // markers of the list of formatting elements).
    Rule {
        ends: &["a"],
        by: &["a"],
        within: &["td", "th", "caption"],
    },
];
