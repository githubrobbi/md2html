// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Raw HTML blocks: the seven kinds `CommonMark` tells apart by their first
//! line, and the line that ends each.

use super::line::{Line, is_blank};
use crate::model::{Block, HtmlBlock};
use crate::scan::{Cursor, Scan};

/// The tags whose content is raw text (`CommonMark`'s first kind).
const RAW_TAGS: [&str; 4] = ["script", "pre", "style", "textarea"];

/// The tags whose blocks end at a blank line (`CommonMark`'s sixth kind).
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// Which of `CommonMark`'s seven kinds an HTML block is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HtmlKind {
    /// `<script>`, `<pre>`, `<style>`, `<textarea>`.
    Raw,
    /// `<!--`.
    Comment,
    /// `<?`.
    Instruction,
    /// `<!` and a letter.
    Declaration,
    /// `<![CDATA[`.
    Cdata,
    /// One of the block tags, open or closing.
    Block,
    /// Any other complete tag alone on its line.
    Tag,
}

/// What ends an HTML block.
#[derive(Clone, Copy, Debug)]
enum HtmlEnd {
    /// The line that holds one of these, in any case; that line belongs
    /// to the block.
    Marks(&'static [&'static str]),
    /// A blank line, which does not.
    BlankLine,
}

impl HtmlKind {
    /// The kind of HTML block `body`, a line without its indentation,
    /// opens.
    pub(super) fn opening(body: &str) -> Option<Self> {
        if !body.starts_with('<') {
            return None;
        }
        let lower = body.to_ascii_lowercase();
        let after_open = lower.strip_prefix('<')?;
        let raw = RAW_TAGS.iter().any(|tag| {
            after_open
                .strip_prefix(tag)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '>']))
        });
        if raw {
            return Some(Self::Raw);
        }
        if after_open.starts_with("!--") {
            return Some(Self::Comment);
        }
        if after_open.starts_with('?') {
            return Some(Self::Instruction);
        }
        if after_open.starts_with("![cdata[") {
            return Some(Self::Cdata);
        }
        if after_open
            .strip_prefix('!')
            .and_then(|rest| rest.chars().next())
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            return Some(Self::Declaration);
        }
        let mut name = Scan::new(after_open.strip_prefix('/').unwrap_or(after_open));
        let tag = name.take_while(|ch| ch.is_ascii_alphanumeric());
        let after = name.rest();
        if BLOCK_TAGS.contains(&tag)
            && (after.is_empty() || after.starts_with([' ', '\t', '>']) || after.starts_with("/>"))
        {
            return Some(Self::Block);
        }
        (!tag.is_empty() && !RAW_TAGS.contains(&tag) && is_lone_tag(body)).then_some(Self::Tag)
    }

    /// Whether a block of this kind may interrupt a paragraph: all but a
    /// lone tag may.
    pub(super) const fn interrupts(self) -> bool {
        !matches!(self, Self::Tag)
    }

    /// What ends a block of this kind.
    const fn end(self) -> HtmlEnd {
        match self {
            Self::Raw => HtmlEnd::Marks(&["</script>", "</pre>", "</style>", "</textarea>"]),
            Self::Comment => HtmlEnd::Marks(&["-->"]),
            Self::Instruction => HtmlEnd::Marks(&["?>"]),
            Self::Declaration => HtmlEnd::Marks(&[">"]),
            Self::Cdata => HtmlEnd::Marks(&["]]>"]),
            Self::Block | Self::Tag => HtmlEnd::BlankLine,
        }
    }
}

/// Whether `body` is one complete open or closing tag and nothing else.
fn is_lone_tag(body: &str) -> bool {
    crate::sanitize::html_tag(body).is_some_and(|tag| {
        !tag.starts_with("<!")
            && !tag.starts_with("<?")
            && body.strip_prefix(tag).is_some_and(is_blank)
    })
}

/// The HTML block of kind `kind` that `head` opens, with the lines of
/// `rest` it runs on to.
pub(super) fn html_block(
    head: &Line<'_>,
    kind: HtmlKind,
    rest: &mut Cursor<'_, Line<'_>>,
) -> Block {
    let more = match kind.end() {
        HtmlEnd::Marks(marks) => {
            let ends = |line: &Line<'_>| {
                let lower = line.text.to_ascii_lowercase();
                marks.iter().any(|mark| lower.contains(mark))
            };
            let spot = rest.spot();
            if !ends(head) {
                while rest.bump().is_some_and(|line| !ends(line)) {}
            }
            rest.since(spot)
        }
        HtmlEnd::BlankLine => rest.take_while(|line| !line.is_blank()),
    };
    let lines: Vec<&str> = core::iter::once(head)
        .chain(more)
        .map(|line| line.text.as_ref())
        .collect();
    Block::Html(HtmlBlock {
        text: lines.join("\n"),
        line: head.no,
    })
}
