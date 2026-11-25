// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a diagram is called, and who called it so.
//!
//! An author titles a diagram in its front matter (`title:` between two
//! `---` lines at the top of the block, Mermaid's way for every kind) or,
//! in a sequence diagram, with a `title` statement. That title is shown
//! under the drawing as its caption. Apart from it, md2html builds a name
//! from the names of a diagram's first parts, for readers who cannot see
//! the drawing, when the author gives none: [`Name`] says which of the two
//! a drawing carries. A title is text: it is escaped where it is written,
//! and costs no more than its own words.

use core::iter::once;

use crate::scan::Scan;

#[cfg(test)]
mod tests;

/// The author's title for a diagram, shown under it as its caption: words,
/// trimmed, never blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::mermaid) struct Caption(String);

impl Caption {
    /// `text` as a caption, trimmed; None when nothing is left of it.
    pub(in crate::mermaid) fn new(text: &str) -> Option<Self> {
        let words = text.trim();
        (!words.is_empty()).then(|| Self(words.to_owned()))
    }

    /// Its words.
    pub(in crate::mermaid) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A diagram's name for readers who cannot see it (the SVG's `<title>`),
/// and who gave it.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::mermaid) enum Name {
    /// The author's: their accessible title (`accTitle:`), or else the
    /// diagram's title.
    Author(String),
    /// Built here from the names of the diagram's first parts.
    Built(String),
}

impl Name {
    /// The author's accessible title `acc` when it says something, else
    /// the diagram's title, else the name `build` makes.
    pub(in crate::mermaid) fn of<Build>(
        acc: Option<&str>,
        caption: Option<&Caption>,
        build: Build,
    ) -> Self
    where
        Build: FnOnce() -> String,
    {
        acc.map(str::trim)
            .filter(|words| !words.is_empty())
            .or_else(|| caption.map(Caption::as_str))
            .map_or_else(
                || Self::Built(build()),
                |words| Self::Author(words.to_owned()),
            )
    }

    /// Its words.
    pub(in crate::mermaid) fn as_str(&self) -> &str {
        match self {
            Self::Author(words) | Self::Built(words) => words,
        }
    }
}

/// A block of Mermaid source with its front matter read off.
#[derive(Debug)]
pub(in crate::mermaid) struct Front<'src> {
    /// The title the front matter gives, if it gives one.
    pub(in crate::mermaid) caption: Option<Caption>,
    /// The diagram after the front matter: the whole source when it has
    /// none.
    pub(in crate::mermaid) body: &'src str,
}

/// The first line of `text`, and what follows it.
fn split_line(text: &str) -> (&str, &str) {
    text.split_once('\n').unwrap_or((text, ""))
}

/// The front matter at the top of a block, and the diagram after it.
///
/// Read the way Mermaid reads it (`extractFrontMatter`): a line of `---`
/// first, blank lines before it aside; YAML; a line of `---`. Of the YAML,
/// md2html reads the top-level `title`; the other keys (`config`,
/// `displayMode`) are Mermaid's own.
///
/// # Errors
///
/// Returns "the front matter has no closing `---`" when the block opens
/// with `---` and no later line is one. The diagram is then shown as code.
pub(in crate::mermaid) fn front(source: &str) -> Result<Front<'_>, String> {
    let (opener, mut rest) = split_line(source.trim_start());
    if opener.trim_end() != "---" {
        return Ok(Front {
            caption: None,
            body: source,
        });
    }
    let mut yaml = Vec::new();
    loop {
        if rest.is_empty() {
            return Err(String::from("the front matter has no closing `---`"));
        }
        let (line, after) = split_line(rest);
        rest = after;
        if line.trim() == "---" {
            break;
        }
        yaml.push(line);
    }
    Ok(Front {
        caption: title(&yaml),
        body: rest,
    })
}

/// How far a line is indented.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The top-level `title` of front matter's YAML lines: a key at the least
/// indentation, its value on its line and on the more indented lines under
/// it.
fn title(yaml: &[&str]) -> Option<Caption> {
    let top = yaml
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| indent(line))
        .min()?;
    let mut lines = yaml.iter();
    while let Some(line) = lines.next() {
        let Some(after) = line.trim().strip_prefix("title:") else {
            continue;
        };
        if indent(line) != top || !(after.is_empty() || after.starts_with(char::is_whitespace)) {
            continue;
        }
        let more: Vec<&str> = lines
            .take_while(|next| next.trim().is_empty() || indent(next) > top)
            .map(|next| next.trim())
            .filter(|next| !next.is_empty())
            .collect();
        return Caption::new(&value(after, &more));
    }
    None
}

/// A YAML value: `first` on the key's line, `more` the lines under it.
///
/// A block scalar (`|`, `>`) is its lines; a quoted one its words without
/// the quotes; a plain one up to a comment. The lines are joined by
/// spaces: a caption is one line.
fn value(first: &str, more: &[&str]) -> String {
    let head = first.trim();
    if head.starts_with(['|', '>']) {
        return more.join(" ");
    }
    let joined = once(head)
        .chain(more.iter().copied())
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    joined.strip_prefix('"').map_or_else(
        || {
            joined
                .strip_prefix('\'')
                .map_or_else(|| plain(&joined), single_quoted)
        },
        double_quoted,
    )
}

/// A plain YAML scalar: its words up to a comment.
fn plain(text: &str) -> String {
    text.split_once(" #")
        .map_or(text, |(words, _)| words)
        .to_owned()
}

/// A double-quoted YAML scalar after its opening quote: up to the quote
/// that closes it, a backslash taking the character after it as written
/// (`\n` and `\t` as a space).
fn double_quoted(inner: &str) -> String {
    let mut scan = Scan::new(inner);
    let mut words = String::new();
    while let Some(ch) = scan.bump() {
        match ch {
            '"' => break,
            '\\' => match scan.bump() {
                Some('n' | 't') => words.push(' '),
                Some(escaped) => words.push(escaped),
                None => {}
            },
            other => words.push(other),
        }
    }
    words
}

/// A single-quoted YAML scalar after its opening quote: up to the quote
/// that closes it, two quotes standing for one.
fn single_quoted(inner: &str) -> String {
    let mut scan = Scan::new(inner);
    let mut words = String::new();
    while let Some(ch) = scan.bump() {
        if ch == '\'' && !scan.eat('\'') {
            break;
        }
        words.push(ch);
    }
    words
}
