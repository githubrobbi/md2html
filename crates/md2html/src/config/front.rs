// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The front matter: the `---` block of `key: value` lines at the very top
//! of a document.

use super::settings::key_name;

/// The most lines a block may run to before a key; past them the opening
/// `---` is a rule, not front matter.
const MOST_LINES: usize = 400;

/// A document's front matter, and the text after it.
#[derive(Debug, Default)]
pub(crate) struct FrontMatter<'text> {
    /// Each key (lowercased, dashes as underscores) with its value, in the
    /// order they were last given: a key given again counts from there,
    /// with its last value.
    pub(crate) pairs: Vec<Pair>,
    /// The Markdown after the block.
    pub(crate) body: &'text str,
    /// The number of the first line of [`Self::body`].
    pub(crate) first_line: usize,
    /// Each key given again after its first time, at the line it is
    /// given again on: its earlier value is not used.
    pub(crate) repeated: Vec<Repeated>,
}

/// One key of the front matter and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pair {
    /// The key, as it is looked up.
    pub(crate) key: String,
    /// Its value, its lines and list items joined.
    pub(crate) value: String,
}

impl FrontMatter<'_> {
    /// The value of `key`, as it is looked up.
    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|pair| pair.key == key)
            .map(|pair| pair.value.as_str())
    }
}

/// A key the front matter gives a second time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Repeated {
    /// The key, as it is looked up.
    pub(crate) key: String,
    /// The line it is given again on, from one.
    pub(crate) line: usize,
}

impl Repeated {
    /// What the author is told.
    pub(crate) fn message(&self) -> String {
        format!(
            "front matter: `{}` is given more than once; the last one is used",
            self.key
        )
    }
}

/// The `---` block at the very top of a document, read; a text without
/// one is all body, from line 1.
pub(crate) fn front_matter(text: &str) -> FrontMatter<'_> {
    let none = FrontMatter {
        body: text,
        first_line: 1,
        ..FrontMatter::default()
    };
    let mut lines = Lines {
        rest: text.strip_prefix('\u{feff}').unwrap_or(text),
    };
    if lines.next_line().map(str::trim_end) != Some("---") {
        return none;
    }
    let mut block = Block::default();
    let mut count = 1_usize;
    while let Some(line) = lines.next_line() {
        count += 1;
        let bare = line.trim_end();
        if bare == "---" || bare == "..." {
            if block.pairs.is_empty() {
                return none;
            }
            return FrontMatter {
                pairs: block.pairs,
                body: lines.rest,
                first_line: count + 1,
                repeated: block.repeated,
            };
        }
        match block.read(bare, count) {
            Step::Taken => {}
            Step::Keyed if count <= MOST_LINES => {}
            Step::Keyed | Step::Refused => return none,
        }
    }
    none
}

/// Text read a line at a time, from the front.
struct Lines<'text> {
    /// What has not been read yet.
    rest: &'text str,
}

impl<'text> Lines<'text> {
    /// The next line, without its line feed; `None` at the end.
    fn next_line(&mut self) -> Option<&'text str> {
        if self.rest.is_empty() {
            return None;
        }
        // The last line may have no line feed.
        let (line, after) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = after;
        Some(line)
    }
}

/// What one line of the block was.
enum Step {
    /// A blank line, a comment, a list item or a continued value.
    Taken,
    /// A `key: value` line.
    Keyed,
    /// Not front matter: the block is a rule and the text that follows.
    Refused,
}

/// The pairs read so far.
#[derive(Default)]
struct Block {
    /// Each key with its value, the last given last: a list item or a
    /// continuation belongs to it.
    pairs: Vec<Pair>,
    /// The keys given again, each at its line.
    repeated: Vec<Repeated>,
}

impl Block {
    /// Reads one line, `line` of the text, its line feed and trailing
    /// spaces taken off.
    fn read(&mut self, bare: &str, line: usize) -> Step {
        let indented = bare.trim_start();
        if indented.is_empty() || indented.starts_with('#') {
            return Step::Taken;
        }
        if let Some(item) = indented.strip_prefix("- ") {
            // A list under the last key: kept as one comma-separated value.
            return self.extend(", ", &unquote(item.trim()));
        }
        if bare.starts_with([' ', '\t']) {
            // A folded continuation of the last value.
            return self.extend(" ", indented);
        }
        self.pair(bare, line)
    }

    /// Adds `piece` to the last key's value, after `separator` when the
    /// value has something already.
    fn extend(&mut self, separator: &str, piece: &str) -> Step {
        let Some(Pair { key: _, value }) = self.pairs.last_mut() else {
            return Step::Refused;
        };
        if !value.is_empty() {
            value.push_str(separator);
        }
        value.push_str(piece);
        Step::Taken
    }

    /// Reads a `key: value` line, `line` of the text.
    fn pair(&mut self, bare: &str, line: usize) -> Step {
        let Some((raw_key, raw_value)) = bare.split_once(':') else {
            return Step::Refused;
        };
        let written = raw_key.trim();
        if written.is_empty()
            || !written
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Step::Refused;
        }
        let key = key_name(written);
        let value = match raw_value.trim() {
            // A block scalar's lines follow as continuations.
            "|" | ">" => String::new(),
            plain => unquote(plain),
        };
        if let Some(earlier) = self.pairs.iter().position(|pair| pair.key == key) {
            self.pairs.remove(earlier);
            self.repeated.push(Repeated {
                key: key.clone(),
                line,
            });
        }
        self.pairs.push(Pair { key, value });
        Step::Keyed
    }
}

/// A value without the quotes around it.
fn unquote(value: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|after_open| after_open.strip_suffix(quote))
        {
            return if quote == '"' {
                inner.replace("\\\"", "\"")
            } else {
                inner.replace("''", "'")
            };
        }
    }
    value.to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Repeated, front_matter};

    /// Front matter is read into settings and taken off the Markdown.
    ///
    /// # Panics
    ///
    /// When a quoted value with a colon is not read whole, when a list is not
    /// joined with commas, when the Markdown after the block is not all that is
    /// left, or when the body is not said to start on line 8.
    #[test]
    fn front_matter_is_read_and_taken_off() {
        let read = front_matter("---\ntitle: \"A: B\"\nlang: de\ntags:\n  - x\n  - y\n---\n# Hi\n");
        assert_eq!(read.get("title"), Some("A: B"));
        assert_eq!(read.get("tags"), Some("x, y"));
        assert_eq!(read.body, "# Hi\n");
        assert_eq!(read.first_line, 8);
        assert_eq!(read.repeated, []);
    }

    /// A `---` rule at the top with no settings after it is not front matter.
    ///
    /// # Panics
    ///
    /// When a `---` followed by a blank line and text is read as settings, or
    /// is taken off the Markdown.
    #[test]
    fn a_rule_at_the_top_is_not_front_matter() {
        let read = front_matter("---\n\nJust text, then a rule.\n");
        assert_eq!(read.pairs, []);
        assert!(read.body.starts_with("---"));
    }

    /// Front matter behind a byte order mark, with CRLF, is read.
    ///
    /// # Panics
    ///
    /// When the byte order mark or the CRLF line ends stop the block from being
    /// read, when the folded `>` value is not joined into one line, when the
    /// key `Brand-Link` is not looked up as `brand_link`, when the `...`
    /// closing line does not end the block, or when the body is not said to
    /// start on line 6.
    #[test]
    fn a_folded_value_and_a_byte_order_mark() {
        let read = front_matter(
            "\u{feff}---\r\nBrand-Link: >\r\n  https://example.com\r\n  /home\r\n...\r\nText",
        );
        assert_eq!(read.get("brand_link"), Some("https://example.com /home"));
        assert_eq!(read.body, "Text");
        assert_eq!(read.first_line, 6);
    }

    /// A closing `---` at the very end of the text ends the front matter.
    ///
    /// # Panics
    ///
    /// When the title before a closing line with no line feed is not read, or
    /// when anything is left as Markdown, or the body is not said to start on
    /// line 4.
    #[test]
    fn a_closing_line_without_a_line_feed_ends_the_block() {
        let read = front_matter("---\ntitle: Hillside\n---");
        assert_eq!(read.get("title"), Some("Hillside"));
        assert_eq!(read.body, "");
        assert_eq!(read.first_line, 4);
    }

    /// A key given twice keeps its last value, and the later line is
    /// named, whatever spelling of the key it was given in.
    ///
    /// # Panics
    ///
    /// When the last `title` is not the one kept, when the second `title`
    /// and `brand_link` given as `Brand-Link` are not named at their lines,
    /// or when a key given once is.
    #[test]
    fn a_key_given_twice_is_named() {
        let read = front_matter(
            "---\ntitle: First\nbrand_link: /a\nlang: de\ntitle: Second\nBrand-Link: /b\n---\nText",
        );
        assert_eq!(read.get("title"), Some("Second"));
        assert_eq!(read.get("brand_link"), Some("/b"));
        assert_eq!(read.repeated, [
            Repeated {
                key: String::from("title"),
                line: 5
            },
            Repeated {
                key: String::from("brand_link"),
                line: 6
            },
        ]);
        assert_eq!(
            read.repeated.first().map(Repeated::message).as_deref(),
            Some("front matter: `title` is given more than once; the last one is used")
        );
    }
}
