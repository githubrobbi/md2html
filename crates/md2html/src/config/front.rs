// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The front matter: the `---` block of `key: value` lines at the very top
//! of a document.

use alloc::collections::BTreeMap;

use super::settings::key_name;

/// The most lines a block may run to before a key; past them the opening
/// `---` is a rule, not front matter.
const MOST_LINES: usize = 400;

/// The `---` block at the very top of a document, as `key: value` pairs
/// (keys lowercased, dashes as underscores), the text after it, and the
/// number of the first line after it.
pub(crate) fn front_matter(text: &str) -> (BTreeMap<String, String>, &str, usize) {
    let none = (BTreeMap::new(), text, 1);
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
            return (block.pairs, lines.rest, count + 1);
        }
        match block.read(bare) {
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
    /// Each key with its value.
    pairs: BTreeMap<String, String>,
    /// The key the next list item or continuation belongs to.
    last_key: Option<String>,
}

impl Block {
    /// Reads one line, its line feed and trailing spaces taken off.
    fn read(&mut self, bare: &str) -> Step {
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
        self.pair(bare)
    }

    /// Adds `piece` to the last key's value, after `separator` when the
    /// value has something already.
    fn extend(&mut self, separator: &str, piece: &str) -> Step {
        let Some(key) = &self.last_key else {
            return Step::Refused;
        };
        let value = self.pairs.entry(key.clone()).or_default();
        if !value.is_empty() {
            value.push_str(separator);
        }
        value.push_str(piece);
        Step::Taken
    }

    /// Reads a `key: value` line.
    fn pair(&mut self, bare: &str) -> Step {
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
        self.pairs.insert(key.clone(), value);
        self.last_key = Some(key);
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
    use super::front_matter;

    #[test]
    fn front_matter_is_read_and_taken_off() {
        let (map, rest, line) =
            front_matter("---\ntitle: \"A: B\"\nlang: de\ntags:\n  - x\n  - y\n---\n# Hi\n");
        assert_eq!(map.get("title").map(String::as_str), Some("A: B"));
        assert_eq!(map.get("tags").map(String::as_str), Some("x, y"));
        assert_eq!(rest, "# Hi\n");
        assert_eq!(line, 8);
    }

    #[test]
    fn a_rule_at_the_top_is_not_front_matter() {
        let (map, rest, _) = front_matter("---\n\nJust text, then a rule.\n");
        assert!(map.is_empty());
        assert!(rest.starts_with("---"));
    }

    #[test]
    fn a_folded_value_and_a_byte_order_mark() {
        let (map, rest, line) = front_matter(
            "\u{feff}---\r\nBrand-Link: >\r\n  https://example.com\r\n  /home\r\n...\r\nText",
        );
        assert_eq!(
            map.get("brand_link").map(String::as_str),
            Some("https://example.com /home")
        );
        assert_eq!(rest, "Text");
        assert_eq!(line, 6);
    }

    #[test]
    fn a_closing_line_without_a_line_feed_ends_the_block() {
        let (map, rest, line) = front_matter("---\ntitle: Hillside\n---");
        assert_eq!(map.get("title").map(String::as_str), Some("Hillside"));
        assert_eq!(rest, "");
        assert_eq!(line, 4);
    }
}
