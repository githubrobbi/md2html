// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A line's end between two Chinese or Japanese characters is no space.
//!
//! Chinese and Japanese put no space between words, so an author who
//! breaks a long line of them in the Markdown means no space there; a
//! browser shows the line's end as one, a stray gap in the sentence. The
//! line's end goes when the characters on both sides of it are East Asian
//! wide (full-width) ones, as CSS's segment break rule, pandoc's
//! `east_asian_line_breaks` and markdown-it's `cjk-breaks` have it. Korean
//! is written with spaces, so Hangul keeps its line's end. Emphasis, a
//! link's text and code are read through, so `日本\n**語**` joins too; any
//! other node between them (an image, a break, HTML, maths) keeps it.

use crate::model::Node;

/// Leaves out every line's end of `nodes` that stands between two wide
/// characters.
pub(super) fn join(nodes: &mut [Node]) {
    let mut read = Read::default();
    read.nodes(nodes);
    if read.drops.contains(&true) {
        let mut drops = read.drops.into_iter();
        leave_out(nodes, &mut drops);
    }
}

/// The first pass: for each line's end in the text, in order, whether it
/// goes.
#[derive(Default)]
struct Read {
    /// One entry per line's end read so far.
    drops: Vec<bool>,
    /// The visible character before the place read to, if the nodes
    /// before it end in one.
    before: Option<char>,
    /// The last line's end read has a wide character before it, and goes
    /// if the next one is wide too.
    waiting: bool,
}

impl Read {
    /// Reads nodes in the order they are shown.
    fn nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Text(text) => self.text(text),
                Node::Code(code) => self.see(code),
                Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => self.nodes(kids),
                Node::Link(link) => self.nodes(&link.kids),
                Node::Image(_)
                | Node::Html(_)
                | Node::Undefined(_)
                | Node::HardBreak
                | Node::Note(_)
                | Node::Math(_) => self.boundary(),
            }
        }
    }

    /// Plain text, with its lines' ends.
    fn text(&mut self, text: &str) {
        let mut rest = text;
        while let Some((line, after)) = rest.split_once('\n') {
            self.see(line);
            self.drops.push(false);
            self.waiting = self.before.is_some_and(wide);
            rest = after;
        }
        self.see(rest);
    }

    /// Visible characters with no line's end among them.
    fn see(&mut self, shown: &str) {
        let mut chars = shown.chars();
        if let Some(first) = chars.next() {
            if self.waiting && wide(first) {
                if let Some(last) = self.drops.last_mut() {
                    *last = true;
                }
            }
            self.waiting = false;
            self.before = Some(chars.next_back().unwrap_or(first));
        }
    }

    /// A node that is no text: what is on either side of it does not meet.
    const fn boundary(&mut self) {
        self.waiting = false;
        self.before = None;
    }
}

/// The second pass: takes out the lines' ends `drops` says go, reading the
/// nodes in the same order as [`Read::nodes`].
fn leave_out(nodes: &mut [Node], drops: &mut impl Iterator<Item = bool>) {
    for node in nodes {
        match node {
            Node::Text(text) => {
                if text.contains('\n') {
                    text.retain(|ch| ch != '\n' || !drops.next().unwrap_or(false));
                }
            }
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => leave_out(kids, drops),
            Node::Link(link) => leave_out(&mut link.kids, drops),
            Node::Code(_)
            | Node::Image(_)
            | Node::Html(_)
            | Node::Undefined(_)
            | Node::HardBreak
            | Node::Note(_)
            | Node::Math(_) => {}
        }
    }
}

/// An East Asian wide or full-width character, or a half-width katakana,
/// that is not Hangul: the scripts and punctuation of Chinese and Japanese.
fn wide(ch: char) -> bool {
    matches!(u32::from(ch),
        0x2E80..=0x2FFF // radicals, Kangxi, ideographic description
        | 0x3000..=0x303F // CJK symbols and punctuation: 、。「」
        | 0x3040..=0x30FF // hiragana, katakana
        | 0x3100..=0x312F // bopomofo
        | 0x3190..=0x33FF // kanbun, bopomofo and katakana extended, strokes, enclosed, compatibility
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF // Han
        | 0xA000..=0xA4CF // Yi
        | 0xFE10..=0xFE1F | 0xFE30..=0xFE6F // vertical, compatibility and small forms
        | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 // full-width forms: （）！？
        | 0xFF61..=0xFF9F // half-width punctuation and katakana
        | 0x1B000..=0x1B16F // kana supplement and extended
        | 0x20000..=0x3FFFD) // Han, the rarer
}

#[cfg(test)]
mod tests {
    use crate::inline::{Env, HtmlMode, parse};
    use crate::model::{Defs, Node};

    /// The text of `src`'s nodes as a page shows them, markup as brackets.
    fn shown(src: &str, html: HtmlMode) -> String {
        /// The nodes' text, each node that holds others in brackets.
        fn walk(nodes: &[Node], out: &mut String) {
            for node in nodes {
                match node {
                    Node::Text(text) | Node::Code(text) => out.push_str(text),
                    Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => {
                        out.push('[');
                        walk(kids, out);
                        out.push(']');
                    }
                    Node::Link(link) => {
                        out.push('[');
                        walk(&link.kids, out);
                        out.push(']');
                    }
                    Node::Image(_)
                    | Node::Html(_)
                    | Node::Undefined(_)
                    | Node::HardBreak
                    | Node::Note(_)
                    | Node::Math(_) => out.push('|'),
                }
            }
        }
        let defs = Defs::default();
        let env = Env {
            defs: &defs,
            html,
            math: true,
        };
        let mut out = String::new();
        walk(&parse(src, &env), &mut out);
        out
    }

    /// Chinese and Japanese text broken over lines reads as one line,
    /// through emphasis, a link's text and code, and after a hard break.
    ///
    /// # Panics
    ///
    /// Panics when a line's end between two wide characters stays, or when
    /// anything else of the text changes.
    #[test]
    fn a_line_s_end_between_two_wide_characters_goes() {
        for (src, want) in [
            ("日本\n語", "日本語"),
            ("これは\nテスト。", "これはテスト。"),
            ("中文，\n继续", "中文，继续"),
            ("句子。  \n下一", "句子。|下一"),
            ("日本 \n  語", "日本語"),
            ("ﾃｽﾄ\nｶﾅ", "ﾃｽﾄｶﾅ"),
            ("日本\n**語**です", "日本[語]です"),
            ("*日本*\n語", "[日本]語"),
            ("[日本](x)\n`語`", "[日本]語"),
            ("一\n二\n三", "一二三"),
        ] {
            assert_eq!(shown(src, HtmlMode::Safe), want, "{src:?}");
        }
    }

    /// Latin text, Korean, a mix of scripts and a line's end beside maths
    /// keep their line's end.
    ///
    /// # Panics
    ///
    /// Panics when a line's end goes that has a character that is not wide
    /// on either side of it, or a node that is no text.
    #[test]
    fn a_line_s_end_stays_unless_both_sides_are_wide() {
        for (src, want) in [
            ("English\nwords", "English\nwords"),
            ("日本\nwords", "日本\nwords"),
            ("words\n日本", "words\n日本"),
            ("한국어\n문장", "한국어\n문장"),
            ("日本\n한국", "日本\n한국"),
            ("日本\n$x$語", "日本\n|語"),
            ("日本 $x$\n語", "日本 |\n語"),
        ] {
            assert_eq!(shown(src, HtmlMode::Safe), want, "{src:?}");
        }
    }

    /// The specification's examples see every line's end as written.
    ///
    /// # Panics
    ///
    /// Panics when the join runs in the `CommonMark` mode.
    #[test]
    fn the_commonmark_tests_keep_every_line_s_end() {
        assert_eq!(shown("日本\n語", HtmlMode::Spec), "日本\n語");
    }
}
