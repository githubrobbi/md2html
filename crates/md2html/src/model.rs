// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The page model: what the block parser (`block.rs`) and the inline
//! parser (`inline.rs`) make of a document, and what the renderer reads.
//!
//! Every kind of block and every compound kind of inline content is a
//! struct of its own, and the enums carry them, so a reader binds the one
//! value it matches and names only the fields it uses. Leaves keep their
//! inline text raw; the inline pass runs once every definition is known.

use alloc::collections::BTreeMap;

/// One block of the document.
#[derive(Debug)]
pub(crate) enum Block {
    /// `#` to `######`, or a paragraph underlined with `===` / `---`.
    Heading(Heading),
    /// Running text.
    Paragraph(Paragraph),
    /// Fenced or indented code.
    Code(CodeBlock),
    /// Raw HTML, as written.
    Html(HtmlBlock),
    /// `---`, `***` or `___`.
    Rule,
    /// `>` lines.
    Quote(Quote),
    /// Bulleted or numbered items.
    List(List),
    /// A pipe table with a delimiter row.
    Table(Table),
    /// A `$$` block.
    Math(MathBlock),
}

impl Block {
    /// The source line the block starts on. A rule and a list have none of
    /// their own: a list's first line is its first item's.
    #[must_use]
    pub(crate) const fn line(&self) -> Option<usize> {
        match self {
            Self::Heading(heading) => Some(heading.line),
            Self::Paragraph(paragraph) => Some(paragraph.line),
            Self::Code(code) => Some(code.line),
            Self::Html(html) => Some(html.line),
            Self::Quote(quote) => Some(quote.line),
            Self::Table(table) => Some(table.line),
            Self::Math(math) => Some(math.line),
            Self::Rule | Self::List(_) => None,
        }
    }
}

/// A heading.
#[derive(Debug)]
pub(crate) struct Heading {
    /// How deep it is.
    pub(crate) level: Level,
    /// The raw inline text.
    pub(crate) text: String,
    /// An explicit `{#id}`.
    pub(crate) id: Option<String>,
    /// How it is written.
    pub(crate) style: HeadingStyle,
    /// The source line.
    pub(crate) line: usize,
}

/// A heading's level: one to six, as HTML has `h1` to `h6`.
///
/// Only [`Level::new`] and the constants make one, so a level is never
/// zero or beyond six, and [`Level::down`] holds it at six.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Level(u8);

impl Level {
    /// The level of a document's title, and of `#`.
    pub(crate) const ONE: Self = Self(1);
    /// The level of a chapter, and of `##`.
    pub(crate) const TWO: Self = Self(2);
    /// The level of a section in a chapter, and of `###`.
    pub(crate) const THREE: Self = Self(3);
    /// The deepest level.
    const SIX: Self = Self(6);

    /// The level `number`; None when it is not from one to six.
    #[must_use]
    pub(crate) const fn new(number: u8) -> Option<Self> {
        if number >= Self::ONE.0 && number <= Self::SIX.0 {
            Some(Self(number))
        } else {
            None
        }
    }

    /// This level `steps` deeper, held at six.
    #[must_use]
    pub(crate) const fn down(self, steps: u8) -> Self {
        let deeper = self.0.saturating_add(steps);
        if deeper > Self::SIX.0 {
            Self::SIX
        } else {
            Self(deeper)
        }
    }
}

impl core::fmt::Display for Level {
    /// The level's number, as `h2` has it.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How a heading is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadingStyle {
    /// `#` marks before the text.
    Atx,
    /// A line of `=` or `-` under the text.
    Setext,
}

/// A paragraph.
#[derive(Debug)]
pub(crate) struct Paragraph {
    /// The raw inline text, lines joined by `\n`.
    pub(crate) text: String,
    /// The source line.
    pub(crate) line: usize,
}

/// A code block, fenced or indented.
#[derive(Debug)]
pub(crate) struct CodeBlock {
    /// The info string after the fence; empty for indented code.
    pub(crate) info: String,
    /// The code, each line ending in `\n`.
    pub(crate) text: String,
    /// The source line.
    pub(crate) line: usize,
}

/// A block of raw HTML.
#[derive(Debug)]
pub(crate) struct HtmlBlock {
    /// The lines, as written.
    pub(crate) text: String,
    /// The source line.
    pub(crate) line: usize,
}

/// A block quote.
#[derive(Debug)]
pub(crate) struct Quote {
    /// What the quote holds.
    pub(crate) blocks: Vec<Block>,
    /// The source line.
    pub(crate) line: usize,
}

/// A list.
#[derive(Debug)]
pub(crate) struct List {
    /// Bulleted, or numbered from a first number.
    pub(crate) kind: ListKind,
    /// Whether a blank line stands between its items.
    pub(crate) spacing: Spacing,
    /// The items.
    pub(crate) items: Vec<Item>,
}

/// How paragraphs are written in the blocks of a list, which `CommonMark`
/// decides by the blank lines between its items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spacing {
    /// No blank line between the items: a paragraph is its bare text.
    Tight,
    /// A blank line between them: a paragraph is wrapped in `<p>`, as
    /// every paragraph outside a list is.
    Loose,
}

/// Whether a list is bulleted or numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// `-`, `+` or `*` items.
    Bullet,
    /// Numbered items, from this first number.
    Ordered(u64),
}

/// One list item.
#[derive(Debug)]
pub(crate) struct Item {
    /// `[ ]` or `[x]` at its start: a task, and whether it is done.
    pub(crate) task: Option<Task>,
    /// What the item holds.
    pub(crate) blocks: Vec<Block>,
}

/// A task item's box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Task {
    /// `[ ]`
    Open,
    /// `[x]`
    Done,
}

/// The kind of a callout: a quote whose first words name one, such as
/// `[!NOTE]` or `**Warning:**`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Callout {
    /// A note.
    Note,
    /// A tip.
    Tip,
    /// Something important.
    Important,
    /// A warning.
    Warning,
    /// A caution.
    Caution,
}

impl Callout {
    /// Every kind.
    pub(crate) const ALL: [Self; 5] = [
        Self::Note,
        Self::Tip,
        Self::Important,
        Self::Warning,
        Self::Caution,
    ];

    /// The kind's class name on the page.
    #[must_use]
    pub(crate) const fn class(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Tip => "tip",
            Self::Important => "important",
            Self::Warning => "warning",
            Self::Caution => "caution",
        }
    }
}

/// A pipe table.
#[derive(Debug)]
pub(crate) struct Table {
    /// One per column.
    pub(crate) align: Vec<Align>,
    /// The header cells, raw.
    pub(crate) head: Vec<String>,
    /// The body rows, raw, as many cells as the header.
    pub(crate) rows: Vec<Vec<String>>,
    /// The source line.
    pub(crate) line: usize,
}

/// A table column's alignment, from its delimiter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    /// `---`
    None,
    /// `:--`
    Left,
    /// `:-:`
    Center,
    /// `--:`
    Right,
}

/// A `$$` maths block.
#[derive(Debug)]
pub(crate) struct MathBlock {
    /// The TeX.
    pub(crate) tex: String,
    /// The source line.
    pub(crate) line: usize,
}

/// What the document defines for later use: link references and
/// footnotes, by normalized label.
#[derive(Default, Debug)]
pub(crate) struct Defs {
    /// `[label]: destination "title"`.
    pub(crate) links: BTreeMap<String, Reference>,
    /// `[^label]: text`.
    pub(crate) notes: BTreeMap<String, Vec<Block>>,
}

/// Where a link reference definition points.
#[derive(Debug)]
pub(crate) struct Reference {
    /// The destination, unescaped.
    pub(crate) href: String,
    /// The title, if any.
    pub(crate) title: Option<String>,
}

/// One piece of inline content.
#[derive(Debug)]
pub(crate) enum Node {
    /// Plain text, not yet escaped.
    Text(String),
    /// A code span's content.
    Code(String),
    /// HTML to write as is (already judged safe, or trusted).
    Html(String),
    /// A line break in the source.
    SoftBreak,
    /// Two spaces or a backslash at a line's end.
    HardBreak,
    /// `*a*`
    Em(Vec<Self>),
    /// `**a**`
    Strong(Vec<Self>),
    /// `~~a~~`
    Del(Vec<Self>),
    /// A link.
    Link(Link),
    /// An image.
    Image(Image),
    /// `[^label]`, a reference to a footnote.
    Note(String),
    /// `$...$` or `$$...$$`.
    Math(Math),
}

impl Node {
    /// The nodes inside: emphasis, a link's text, an image's description;
    /// None for a leaf.
    #[must_use]
    pub(crate) fn kids(&self) -> Option<&[Self]> {
        match self {
            Self::Em(kids) | Self::Strong(kids) | Self::Del(kids) => Some(kids),
            Self::Link(link) => Some(&link.kids),
            Self::Image(image) => Some(&image.alt),
            Self::Text(_)
            | Self::Code(_)
            | Self::Html(_)
            | Self::SoftBreak
            | Self::HardBreak
            | Self::Note(_)
            | Self::Math(_) => None,
        }
    }
}

/// A link.
#[derive(Debug)]
pub(crate) struct Link {
    /// The destination, unescaped.
    pub(crate) href: String,
    /// The title, if any.
    pub(crate) title: Option<String>,
    /// The text.
    pub(crate) kids: Vec<Node>,
}

/// An image.
#[derive(Debug)]
pub(crate) struct Image {
    /// The source, unescaped.
    pub(crate) src: String,
    /// The title, if any.
    pub(crate) title: Option<String>,
    /// The description.
    pub(crate) alt: Vec<Node>,
    /// `{...}` after it.
    pub(crate) attrs: Attrs,
}

/// An image's `{width=50% .class #id}`.
#[derive(Debug, Default, Clone)]
pub(crate) struct Attrs {
    /// `width=`
    pub(crate) width: Option<String>,
    /// `height=`
    pub(crate) height: Option<String>,
    /// `#id`
    pub(crate) id: Option<String>,
    /// `.class`
    pub(crate) classes: Vec<String>,
}

/// Maths inside a line of text.
#[derive(Debug)]
pub(crate) struct Math {
    /// The TeX.
    pub(crate) tex: String,
    /// In the line, or set apart as display maths.
    pub(crate) mode: MathMode,
}

/// How maths is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MathMode {
    /// `$...$`, in the line.
    Inline,
    /// `$$...$$`, set apart.
    Display,
}

#[cfg(test)]
mod tests {
    use super::Level;

    /// A level is one to six, and moving down holds it at six.
    ///
    /// # Panics
    ///
    /// When zero or seven is a level, when one to six is not, or when a
    /// level moved down goes past six or is not written as its number.
    #[test]
    fn a_level_is_one_to_six() {
        assert_eq!(Level::new(0), None);
        assert_eq!(Level::new(7), None);
        for number in 1..=6 {
            let level = Level::new(number).expect("one to six");
            assert_eq!(level.to_string(), number.to_string());
        }
        assert_eq!(Level::ONE.down(1), Level::TWO);
        assert_eq!(Level::THREE.down(200), Level::new(6).expect("six"));
        assert!(Level::ONE < Level::TWO && Level::TWO < Level::THREE);
    }
}
