// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Inline text, the way `CommonMark` reads it: code spans first, then a
//! delimiter stack for emphasis and brackets, so `***both***` nests,
//! `[x]` stays text unless it is a link, and a URL may hold parentheses.
//! GitHub's additions come along: `~~strike~~`, bare URLs, `[^note]`
//! references; and `$maths$`.

use crate::block::{Defs, normalize_label};

/// How raw HTML in the text is treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HtmlMode {
    /// A short list of harmless tags passes; anything else is shown as written.
    Safe,
    /// The author's HTML passes as is (`--html`).
    Trusted,
    /// Everything passes, as the `CommonMark` tests expect.
    Spec,
}

/// What the inline pass needs to know about the document.
pub(crate) struct Env<'a> {
    /// The link references and footnotes.
    pub(crate) defs: &'a Defs,
    /// Raw HTML policy.
    pub(crate) html: HtmlMode,
    /// `$...$` is maths.
    pub(crate) math: bool,
}

/// An image's or link's `{width=50% .class #id}`.
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
    Em(Vec<Node>),
    /// `**a**`
    Strong(Vec<Node>),
    /// `~~a~~`
    Del(Vec<Node>),
    /// A link.
    Link {
        /// The destination, unescaped.
        href: String,
        /// The title, if any.
        title: Option<String>,
        /// The text.
        kids: Vec<Node>,
    },
    /// An image.
    Image {
        /// The source, unescaped.
        src: String,
        /// The title, if any.
        title: Option<String>,
        /// The description.
        alt: Vec<Node>,
        /// `{...}` after it.
        attrs: Attrs,
    },
    /// `[^label]`, a reference to a footnote.
    Note(String),
    /// `$...$` or `$$...$$`.
    Math {
        /// The TeX.
        tex: String,
        /// Display maths.
        display: bool,
    },
}

/// Inline parse state: tokens with delimiters still unresolved.
enum Tok {
    /// A finished node.
    N(Node),
    /// A run of `*`, `_` or `~`.
    D {
        /// The mark.
        ch: char,
        /// How many are left.
        n: usize,
        /// How many there were.
        orig: usize,
        /// May open emphasis.
        open: bool,
        /// May close emphasis.
        close: bool,
        /// Could both open and close, as written (the rule of three).
        both: bool,
    },
    /// `[` or `![`.
    B {
        /// `![`.
        image: bool,
        /// Still allowed to become a link.
        active: bool,
        /// Where its text starts in the source.
        pos: usize,
    },
}

/// Whether a character counts as punctuation for emphasis (`CommonMark`
/// 0.31 counts symbols too).
fn is_punct(ch: char) -> bool {
    ch.is_ascii_punctuation() || (!ch.is_alphanumeric() && !ch.is_whitespace() && !ch.is_ascii())
}

/// The inline nodes of `src`.
pub(crate) fn parse(src: &str, env: &Env<'_>) -> Vec<Node> {
    Inline {
        src,
        env,
        toks: Vec::new(),
        text: String::new(),
    }
    .run()
}

/// One inline parse.
struct Inline<'s, 'e> {
    /// The text.
    src: &'s str,
    /// The document's definitions and policies.
    env: &'e Env<'e>,
    /// The tokens so far.
    toks: Vec<Tok>,
    /// Plain text not yet pushed as a token.
    text: String,
}

impl Inline<'_, '_> {
    /// Pushes the pending text as a node.
    fn flush(&mut self) {
        if !self.text.is_empty() {
            let text = core::mem::take(&mut self.text);
            self.toks.push(Tok::N(Node::Text(text)));
        }
    }

    /// Pushes a finished node.
    fn node(&mut self, node: Node) {
        self.flush();
        self.toks.push(Tok::N(node));
    }

    /// The character before byte `at`, or a space at the start.
    fn before(&self, at: usize) -> char {
        self.src
            .get(..at)
            .and_then(|head| head.chars().next_back())
            .unwrap_or(' ')
    }

    /// The character at byte `at`, or a space at the end.
    fn at(&self, at: usize) -> char {
        self.src
            .get(at..)
            .and_then(|tail| tail.chars().next())
            .unwrap_or(' ')
    }

    fn run(mut self) -> Vec<Node> {
        let src = self.src;
        let bytes = src.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;
        while i < len {
            let byte = bytes[i];
            match byte {
                b'\\' => {
                    let next = self.at(i + 1);
                    if i + 1 < len && next.is_ascii_punctuation() {
                        self.text.push(next);
                        i += 2;
                    } else if i + 1 < len && next == '\n' {
                        self.node(Node::HardBreak);
                        i += 2;
                        while i < len && bytes[i] == b' ' {
                            i += 1;
                        }
                    } else {
                        self.text.push('\\');
                        i += 1;
                    }
                }
                b'`' => {
                    let run = bytes[i..].iter().take_while(|b| **b == b'`').count();
                    if let Some((code, end)) = code_span(src, i, run) {
                        self.node(Node::Code(code));
                        i = end;
                    } else {
                        self.text.push_str(&src[i..i + run]);
                        i += run;
                    }
                }
                b'*' | b'_' | b'~' => {
                    let ch = char::from(byte);
                    let run = bytes[i..].iter().take_while(|b| **b == byte).count();
                    let prev = self.before(i);
                    let next = self.at(i + run);
                    if ch == '~' && run != 2 {
                        self.text.push_str(&src[i..i + run]);
                        i += run;
                        continue;
                    }
                    let left = !next.is_whitespace()
                        && (!is_punct(next) || prev.is_whitespace() || is_punct(prev));
                    let right = !prev.is_whitespace()
                        && (!is_punct(prev) || next.is_whitespace() || is_punct(next));
                    let (open, close) = if ch == '_' {
                        (
                            left && (!right || is_punct(prev)),
                            right && (!left || is_punct(next)),
                        )
                    } else {
                        (left, right)
                    };
                    self.flush();
                    self.toks.push(Tok::D {
                        ch,
                        n: run,
                        orig: run,
                        open,
                        close,
                        both: open && close,
                    });
                    i += run;
                }
                b'!' if bytes.get(i + 1) == Some(&b'[') => {
                    self.flush();
                    self.toks.push(Tok::B {
                        image: true,
                        active: true,
                        pos: i + 2,
                    });
                    i += 2;
                }
                b'[' => {
                    self.flush();
                    self.toks.push(Tok::B {
                        image: false,
                        active: true,
                        pos: i + 1,
                    });
                    i += 1;
                }
                b']' => {
                    i = self.close_bracket(i);
                }
                b'<' => {
                    if let Some((href, text, end)) = autolink(src, i) {
                        self.node(Node::Link {
                            href,
                            title: None,
                            kids: vec![Node::Text(text)],
                        });
                        i = end;
                    } else if let Some(taglen) = html_tag(&src[i..]) {
                        let tag = &src[i..i + taglen];
                        match self.env.html {
                            HtmlMode::Trusted | HtmlMode::Spec => {
                                self.node(Node::Html(tag.to_owned()));
                            }
                            HtmlMode::Safe => {
                                if tag.starts_with("<!--") {
                                    // A comment is the author's note to self: never shown.
                                } else if let Some(clean) = safe_tag(tag) {
                                    self.node(Node::Html(clean));
                                } else {
                                    self.text.push_str(tag);
                                }
                            }
                        }
                        i += taglen;
                    } else {
                        self.text.push('<');
                        i += 1;
                    }
                }
                b'&' => {
                    if let Some((decoded, used)) = entity(&src[i..]) {
                        match decoded {
                            Entity::Char(ch) => self.text.push(ch),
                            Entity::Raw(raw) => self.node(Node::Html(raw)),
                        }
                        i += used;
                    } else {
                        self.text.push('&');
                        i += 1;
                    }
                }
                b'\n' => {
                    let trailing = self.text.len() - self.text.trim_end_matches(' ').len();
                    let hard = trailing >= 2;
                    let keep = self.text.trim_end_matches(' ').len();
                    self.text.truncate(keep);
                    self.node(if hard {
                        Node::HardBreak
                    } else {
                        Node::SoftBreak
                    });
                    i += 1;
                    while i < len && bytes[i] == b' ' {
                        i += 1;
                    }
                }
                b'$' if self.env.math => {
                    if let Some((tex, display, end)) = math_span(src, i) {
                        self.node(Node::Math { tex, display });
                        i = end;
                    } else {
                        self.text.push('$');
                        i += 1;
                    }
                }
                b'h' | b'w' | b'H' | b'W' => {
                    let prev = self.before(i);
                    let boundary = self.env.html != HtmlMode::Spec
                        && (prev.is_whitespace()
                            || matches!(prev, '*' | '_' | '~' | '(' | '"' | '\''));
                    if let Some((href, end)) = boundary.then(|| bare_url(src, i)).flatten() {
                        let shown = src[i..end].to_owned();
                        self.node(Node::Link {
                            href,
                            title: None,
                            kids: vec![Node::Text(shown)],
                        });
                        i = end;
                    } else {
                        self.text.push(char::from(byte));
                        i += 1;
                    }
                }
                _ => {
                    let ch = self.at(i);
                    self.text.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        self.flush();
        // A trailing hard break is not one.
        while matches!(
            self.toks.last(),
            Some(Tok::N(Node::HardBreak | Node::SoftBreak))
        ) {
            self.toks.pop();
        }
        let toks = core::mem::take(&mut self.toks);
        merge(emphasis(toks))
    }

    /// A `]`: a link or image if the bracket before it and what follows
    /// make one; text otherwise. Returns where to go on.
    fn close_bracket(&mut self, at: usize) -> usize {
        let src = self.src;
        let Some(open_at) = self
            .toks
            .iter()
            .rposition(|tok| matches!(tok, Tok::B { .. }))
        else {
            self.text.push(']');
            return at + 1;
        };
        let Tok::B { image, active, pos } = self.toks[open_at] else {
            return at + 1;
        };
        if !active {
            self.toks[open_at] = Tok::N(Node::Text(String::from(if image { "![" } else { "[" })));
            self.text.push(']');
            return at + 1;
        }
        let label_text = src.get(pos..at).unwrap_or_default();
        let after = at + 1;
        // A footnote reference: `[^label]`.
        if !image {
            if let Some(name) = label_text.strip_prefix('^') {
                let key = normalize_label(name);
                if !key.is_empty()
                    && self.env.defs.notes.contains_key(&key)
                    && src.get(after..after + 1) != Some("(")
                {
                    // The label's text is still pending; it is the reference, not prose.
                    self.text.clear();
                    self.toks.truncate(open_at);
                    self.node(Node::Note(key));
                    return after;
                }
            }
        }
        let mut target: Option<(String, Option<String>, usize)> = None;
        let mut followed_by_label = false;
        if src.get(after..after + 1) == Some("(") {
            target = inline_target(src, after);
        }
        if target.is_none() {
            if src.get(after..after + 1) == Some("[") {
                if let Some(close) = src.get(after + 1..).and_then(label_close) {
                    let second = &src[after + 1..after + 1 + close];
                    let key = if second.trim().is_empty() {
                        normalize_label(label_text)
                    } else {
                        normalize_label(second)
                    };
                    if let Some((href, title)) = self.env.defs.links.get(&key) {
                        target = Some((href.clone(), title.clone(), after + close + 2));
                    }
                    followed_by_label = true;
                }
            }
            if target.is_none() && !followed_by_label {
                if let Some((href, title)) = self.env.defs.links.get(&normalize_label(label_text)) {
                    if !label_text.trim().is_empty() {
                        target = Some((href.clone(), title.clone(), after));
                    }
                }
            }
        }
        let Some((href, title, mut end)) = target else {
            self.toks[open_at] = Tok::N(Node::Text(String::from(if image { "![" } else { "[" })));
            self.text.push(']');
            return at + 1;
        };
        self.flush();
        let inner: Vec<Tok> = self.toks.split_off(open_at + 1);
        self.toks.pop();
        let kids = merge(emphasis(inner));
        if image {
            let mut attrs = Attrs::default();
            if src.get(end..end + 1) == Some("{") {
                if let Some((parsed, used)) = attributes(&src[end..]) {
                    attrs = parsed;
                    end += used;
                }
            }
            self.toks.push(Tok::N(Node::Image {
                src: href,
                title,
                alt: kids,
                attrs,
            }));
        } else {
            for tok in &mut self.toks {
                if let Tok::B {
                    image: false,
                    active,
                    ..
                } = tok
                {
                    *active = false;
                }
            }
            self.toks.push(Tok::N(Node::Link {
                href,
                title,
                kids: unlink(kids),
            }));
        }
        end
    }
}

/// A link's text never holds another link: a bare URL inside it is text.
fn unlink(kids: Vec<Node>) -> Vec<Node> {
    kids.into_iter()
        .map(|node| match node {
            Node::Link { kids, .. } => Node::Text(plain(&kids)),
            other => other,
        })
        .collect()
}

/// Emphasis from the delimiter runs: `CommonMark`'s "process emphasis",
/// rule of three included.
fn emphasis(mut toks: Vec<Tok>) -> Vec<Node> {
    let mut c = 0usize;
    while c < toks.len() {
        let Tok::D {
            ch: cch,
            n: cn,
            orig: corig,
            both: cboth,
            close: true,
            ..
        } = toks[c]
        else {
            c += 1;
            continue;
        };
        let mut found = None;
        for o in (0..c).rev() {
            if let Tok::D {
                ch,
                n,
                orig,
                open: true,
                both: oboth,
                ..
            } = toks[o]
            {
                if ch != cch || n == 0 {
                    continue;
                }
                if (cboth || oboth) && (orig + corig) % 3 == 0 && !(orig % 3 == 0 && corig % 3 == 0)
                {
                    continue;
                }
                if cch == '~' && (n < 2 || cn < 2) {
                    continue;
                }
                found = Some((o, n));
                break;
            }
        }
        let Some((o, on)) = found else {
            if let Tok::D { close, .. } = &mut toks[c] {
                *close = false;
            }
            c += 1;
            continue;
        };
        let used = if cch == '~' || (on >= 2 && cn >= 2) {
            2
        } else {
            1
        };
        let inner: Vec<Tok> = toks.drain(o + 1..c).collect();
        let kids = merge(plain_toks(inner));
        let node = match (cch, used) {
            ('~', _) => Node::Del(kids),
            (_, 2) => Node::Strong(kids),
            _ => Node::Em(kids),
        };
        if let Tok::D { n, .. } = &mut toks[o] {
            *n -= used;
        }
        if let Tok::D { n, .. } = &mut toks[o + 1] {
            *n -= used;
        }
        toks.insert(o + 1, Tok::N(node));
        let mut closer = o + 2;
        if matches!(toks[o], Tok::D { n: 0, .. }) {
            toks.remove(o);
            closer -= 1;
        }
        if matches!(toks[closer], Tok::D { n: 0, .. }) {
            toks.remove(closer);
        }
        c = closer;
    }
    plain_toks(toks)
}

/// Tokens as nodes: delimiters and brackets left over are text.
fn plain_toks(toks: Vec<Tok>) -> Vec<Node> {
    toks.into_iter()
        .filter_map(|tok| match tok {
            Tok::N(node) => Some(node),
            Tok::D { ch, n, .. } => {
                (n > 0).then(|| Node::Text(core::iter::repeat_n(ch, n).collect()))
            }
            Tok::B { image, .. } => Some(Node::Text(String::from(if image { "![" } else { "[" }))),
        })
        .collect()
}

/// Neighbouring text nodes as one.
fn merge(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if let Node::Text(text) = &node {
            if let Some(Node::Text(last)) = out.last_mut() {
                last.push_str(text);
                continue;
            }
        }
        out.push(node);
    }
    out
}

/// A code span opened by `run` backticks at `at`: its content and where
/// it ends.
fn code_span(src: &str, at: usize, run: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    let mut j = at + run;
    while j < bytes.len() {
        if bytes[j] == b'`' {
            let close = bytes[j..].iter().take_while(|b| **b == b'`').count();
            if close == run {
                let mut code = src[at + run..j].replace('\n', " ");
                if code.len() >= 2
                    && code.starts_with(' ')
                    && code.ends_with(' ')
                    && !code.trim().is_empty()
                {
                    code = code[1..code.len() - 1].to_owned();
                }
                return Some((code, j + close));
            }
            j += close;
        } else {
            j += 1;
        }
    }
    None
}

/// `(destination "title")` after a `]` at `at`: destination, title, end.
fn inline_target(src: &str, at: usize) -> Option<(String, Option<String>, usize)> {
    let mut j = at + 1;
    let skip = |j: &mut usize| {
        let mut newlines = 0;
        while let Some(ch) = src.get(*j..).and_then(|t| t.chars().next()) {
            if ch == '\n' {
                newlines += 1;
                if newlines > 1 {
                    break;
                }
            } else if ch != ' ' && ch != '\t' {
                break;
            }
            *j += ch.len_utf8();
        }
    };
    skip(&mut j);
    let rest = src.get(j..)?;
    let (dest, used) = if rest.starts_with(')') {
        (String::new(), 0)
    } else {
        link_destination(rest)?
    };
    j += used;
    let before_title = j;
    skip(&mut j);
    let mut title = None;
    if j > before_title || used == 0 {
        if let Some((t, tlen)) = src.get(j..).and_then(link_title) {
            title = Some(t);
            j += tlen;
            skip(&mut j);
        }
    }
    (src.get(j..j + 1) == Some(")")).then(|| (dest, title, j + 1))
}

/// A link destination at the start of `text`: unescaped, and its length.
pub(crate) fn link_destination(text: &str) -> Option<(String, usize)> {
    if let Some(rest) = text.strip_prefix('<') {
        let mut escaped = false;
        for (at, ch) in rest.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '\n' | '<' => return None,
                '>' => return Some((unescape(&rest[..at]), at + 2)),
                _ => {}
            }
        }
        return None;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    let mut end = text.len();
    for (at, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '(' => {
                depth += 1;
                if depth > 32 {
                    return None;
                }
            }
            ')' => {
                if depth == 0 {
                    end = at;
                    break;
                }
                depth -= 1;
            }
            c if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c.is_ascii_control() => {
                end = at;
                break;
            }
            _ => {}
        }
    }
    if end == 0 || depth != 0 {
        return None;
    }
    Some((unescape(&text[..end]), end))
}

/// A link title at the start of `text`: unescaped, and its length.
pub(crate) fn link_title(text: &str) -> Option<(String, usize)> {
    let open = text.chars().next()?;
    let close = match open {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return None,
    };
    let mut escaped = false;
    for (at, ch) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == close {
            return Some((unescape(&text[1..at]), at + 1));
        } else if open == '(' && ch == '(' {
            return None;
        }
    }
    None
}

/// Where a label's closing `]` is, in text after its `[`.
fn label_close(text: &str) -> Option<usize> {
    let mut escaped = false;
    for (at, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' => return None,
            ']' => return (at <= 999).then_some(at),
            _ => {}
        }
    }
    None
}

/// `{width=50% .wide #map}` after an image: what it says and its length.
fn attributes(text: &str) -> Option<(Attrs, usize)> {
    let close = text.find('}')?;
    let inner = text.get(1..close)?;
    if inner.contains('\n') || inner.trim().is_empty() {
        return None;
    }
    let mut attrs = Attrs::default();
    for piece in inner.split_whitespace() {
        let ok_value = |v: &str| {
            !v.is_empty()
                && v.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '%'))
        };
        if let Some(id) = piece.strip_prefix('#') {
            attrs.id = Some(id.to_owned());
        } else if let Some(class) = piece.strip_prefix('.') {
            attrs.classes.push(class.to_owned());
        } else if let Some(v) = piece.strip_prefix("width=") {
            attrs.width = ok_value(v.trim_matches('"')).then(|| v.trim_matches('"').to_owned());
        } else {
            let v = piece.strip_prefix("height=")?;
            attrs.height = ok_value(v.trim_matches('"')).then(|| v.trim_matches('"').to_owned());
        }
    }
    let ident = |s: &str| {
        s.chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_'))
    };
    if attrs.id.as_deref().is_some_and(|id| !ident(id)) || attrs.classes.iter().any(|c| !ident(c)) {
        return None;
    }
    Some((attrs, close + 1))
}

/// `<scheme:...>` or `<user@host>` at `at`: href, text, end.
fn autolink(src: &str, at: usize) -> Option<(String, String, usize)> {
    let rest = src.get(at + 1..)?;
    let close = rest.find('>')?;
    let inner = &rest[..close];
    if inner.is_empty()
        || inner.contains(|ch: char| ch.is_whitespace() || ch == '<' || ch.is_control())
    {
        return None;
    }
    if let Some((scheme, _tail)) = inner.split_once(':') {
        let ok = (2..=32).contains(&scheme.len())
            && scheme.starts_with(|ch: char| ch.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'));
        if ok {
            return Some((inner.to_owned(), inner.to_owned(), at + close + 2));
        }
    }
    let (user, host) = inner.split_once('@')?;
    let user_ok = !user.is_empty()
        && user
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(ch));
    let host_ok = !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        });
    (user_ok && host_ok).then(|| (format!("mailto:{inner}"), inner.to_owned(), at + close + 2))
}

/// A bare `https://`, `http://` or `www.` link at `at`: href and end.
fn bare_url(src: &str, at: usize) -> Option<(String, usize)> {
    let rest = &src[at..];
    let lower: String = rest
        .chars()
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    let (prefix, www) = if lower.starts_with("https://") {
        (8, false)
    } else if lower.starts_with("http://") {
        (7, false)
    } else if lower.starts_with("www.") {
        (4, true)
    } else {
        return None;
    };
    let mut end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '<')
        .unwrap_or(rest.len());
    let host_end = rest[prefix..end]
        .find(['/', '?', '#'])
        .map_or(end, |p| p + prefix);
    let host = &rest[prefix..host_end];
    if host.is_empty()
        || !host
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        || (www && !host.contains('.'))
    {
        return None;
    }
    loop {
        let shown = &rest[..end];
        let Some(last) = shown.chars().next_back() else {
            break;
        };
        if "?!.,:*_~'\"".contains(last) {
            end -= last.len_utf8();
        } else if last == ')' && shown.matches(')').count() > shown.matches('(').count() {
            end -= 1;
        } else if last == ';' {
            match shown.rfind('&') {
                Some(amp)
                    if shown[amp + 1..shown.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric()) =>
                {
                    end = amp;
                }
                _ => break,
            }
        } else {
            break;
        }
    }
    if end <= prefix {
        return None;
    }
    let shown = &rest[..end];
    let href = if www {
        format!("http://{shown}")
    } else {
        shown.to_owned()
    };
    Some((href, at + end))
}

/// `$tex$` or `$$tex$$` at `at`, pandoc's rules: no space just inside the
/// marks, and no digit right after the closing one, so "$5 and $10" is
/// money. Returns the TeX, whether it is display maths, and the end.
fn math_span(src: &str, at: usize) -> Option<(String, bool, usize)> {
    let rest = &src[at..];
    if let Some(tail) = rest.strip_prefix("$$") {
        let close = tail.find("$$")?;
        let tex = tail[..close].trim();
        return (!tex.is_empty()).then(|| (tex.to_owned(), true, at + 2 + close + 2));
    }
    let tail = &rest[1..];
    if tail.starts_with(char::is_whitespace) {
        return None;
    }
    let mut escaped = false;
    for (pos, ch) in tail.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '\n' if tail[..pos].ends_with('\n') => return None,
            '$' => {
                let before = tail[..pos].chars().next_back()?;
                let after = tail[pos + 1..].chars().next();
                if before.is_whitespace() || after.is_some_and(|c| c.is_ascii_digit()) {
                    return None;
                }
                return Some((tail[..pos].to_owned(), false, at + 1 + pos + 1));
            }
            _ => {}
        }
    }
    None
}

/// The length of a complete HTML tag, comment, processing instruction,
/// declaration or CDATA section at the start of `text`, if one is there.
pub(crate) fn html_tag(text: &str) -> Option<usize> {
    let rest = text.strip_prefix('<')?;
    if let Some(body) = rest.strip_prefix("!--") {
        if body.starts_with('>') {
            return Some(5);
        }
        if body.starts_with("->") {
            return Some(6);
        }
        return body.find("-->").map(|end| 4 + end + 3);
    }
    if let Some(body) = rest.strip_prefix('?') {
        return body.find("?>").map(|end| 2 + end + 2);
    }
    if let Some(body) = rest.strip_prefix("![CDATA[") {
        return body.find("]]>").map(|end| 9 + end + 3);
    }
    if let Some(body) = rest.strip_prefix('!') {
        if body.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
            return body.find('>').map(|end| 2 + end + 1);
        }
        return None;
    }
    let closing = rest.starts_with('/');
    let body = if closing { &rest[1..] } else { rest };
    let name_len = body
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .count();
    if name_len == 0 || !body.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
        return None;
    }
    let mut j = name_len;
    let b = body.as_bytes();
    let space = |j: &mut usize| {
        let s = *j;
        while *j < b.len() && matches!(b[*j], b' ' | b'\t' | b'\n') {
            *j += 1;
        }
        *j > s
    };
    if closing {
        space(&mut j);
        return (b.get(j) == Some(&b'>')).then(|| 1 + 1 + j + 1);
    }
    loop {
        let had_space = space(&mut j);
        match b.get(j) {
            Some(b'>') => return Some(1 + j + 1),
            Some(b'/') => return (b.get(j + 1) == Some(&b'>')).then(|| 1 + j + 2),
            Some(ch) if had_space && (ch.is_ascii_alphabetic() || *ch == b'_' || *ch == b':') => {
                while j < b.len()
                    && (b[j].is_ascii_alphanumeric() || matches!(b[j], b'_' | b'.' | b':' | b'-'))
                {
                    j += 1;
                }
                let save = j;
                space(&mut j);
                if b.get(j) == Some(&b'=') {
                    j += 1;
                    space(&mut j);
                    match b.get(j) {
                        Some(q @ (b'"' | b'\'')) => {
                            let q = *q;
                            j += 1;
                            while j < b.len() && b[j] != q {
                                j += 1;
                            }
                            if j >= b.len() {
                                return None;
                            }
                            j += 1;
                        }
                        Some(_) => {
                            let s = j;
                            while j < b.len() && !b" \t\n\"'=<>`".contains(&b[j]) {
                                j += 1;
                            }
                            if j == s {
                                return None;
                            }
                        }
                        None => return None,
                    }
                } else {
                    j = save;
                }
            }
            _ => return None,
        }
    }
}

/// The tags safe to pass through from an untrusted page, and what each
/// may carry. `<details>`, `<kbd>`, `<sub>`, `<br>` and the like are what
/// Markdown writers reach for; scripts, styles, frames, forms and event
/// handlers are not among them.
const SAFE_TAGS: &[(&str, &[&str])] = &[
    ("abbr", &["title"]),
    ("b", &[]),
    ("br", &[]),
    ("cite", &[]),
    ("code", &[]),
    ("dd", &[]),
    ("del", &[]),
    ("details", &["open"]),
    ("dfn", &[]),
    ("div", &[]),
    ("dl", &[]),
    ("dt", &[]),
    ("em", &[]),
    ("hr", &[]),
    ("i", &[]),
    ("ins", &[]),
    ("kbd", &[]),
    ("li", &[]),
    ("mark", &[]),
    ("ol", &["start"]),
    ("p", &[]),
    ("q", &[]),
    ("s", &[]),
    ("samp", &[]),
    ("small", &[]),
    ("span", &[]),
    ("strong", &[]),
    ("sub", &[]),
    ("summary", &[]),
    ("sup", &[]),
    ("table", &[]),
    ("tbody", &[]),
    ("td", &["colspan", "rowspan"]),
    ("tfoot", &[]),
    ("th", &["colspan", "rowspan"]),
    ("thead", &[]),
    ("tr", &[]),
    ("u", &[]),
    ("ul", &[]),
    ("var", &[]),
    ("wbr", &[]),
];

/// A tag rewritten with only its allowed attributes, or None when the tag
/// is not on the safe list (it is then shown as written).
pub(crate) fn safe_tag(tag: &str) -> Option<String> {
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let inner = inner.strip_suffix('/').unwrap_or(inner);
    let name_len = inner
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .count();
    let name = inner.get(..name_len)?.to_ascii_lowercase();
    let (_, allowed) = SAFE_TAGS.iter().find(|(tag_name, _)| *tag_name == name)?;
    if closing {
        return Some(format!("</{name}>"));
    }
    let mut out = format!("<{name}");
    let mut rest = inner.get(name_len..).unwrap_or_default().trim();
    while !rest.is_empty() {
        let key_len = rest
            .find(|ch: char| ch == '=' || ch.is_whitespace())
            .unwrap_or(rest.len());
        let key = rest[..key_len].to_ascii_lowercase();
        rest = rest[key_len..].trim_start();
        let mut value: Option<String> = None;
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (v, used) = if let Some(q @ ('"' | '\'')) = after.chars().next() {
                let end = after[1..].find(q).map_or(after.len(), |e| e + 1);
                (
                    after.get(1..end).unwrap_or_default().to_owned(),
                    (end + 1).min(after.len()),
                )
            } else {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                (after[..end].to_owned(), end)
            };
            value = Some(v);
            rest = after[used..].trim_start();
        }
        if allowed.contains(&key.as_str()) {
            out.push(' ');
            out.push_str(&key);
            if let Some(v) = value {
                out.push_str("=\"");
                out.push_str(&crate::esc(&decode_entities(&v)));
                out.push('"');
            }
        }
    }
    out.push('>');
    Some(out)
}

/// What an entity reference stands for.
enum Entity {
    /// A character this tool knows.
    Char(char),
    /// A well-formed named reference left for the browser to decode.
    Raw(String),
}

/// The common named entities, so the search index and the alt texts see
/// the character rather than its name.
const ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("copy", '©'),
    ("reg", '®'),
    ("trade", '™'),
    ("hellip", '…'),
    ("mdash", '—'),
    ("ndash", '–'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("laquo", '«'),
    ("raquo", '»'),
    ("bdquo", '„'),
    ("sbquo", '‚'),
    ("times", '×'),
    ("divide", '÷'),
    ("deg", '°'),
    ("plusmn", '±'),
    ("middot", '·'),
    ("bull", '•'),
    ("euro", '€'),
    ("pound", '£'),
    ("yen", '¥'),
    ("cent", '¢'),
    ("sect", '§'),
    ("para", '¶'),
    ("frac12", '½'),
    ("frac14", '¼'),
    ("frac34", '¾'),
    ("larr", '←'),
    ("rarr", '→'),
    ("uarr", '↑'),
    ("darr", '↓'),
    ("harr", '↔'),
    ("rArr", '⇒'),
    ("lArr", '⇐'),
    ("hArr", '⇔'),
    ("le", '≤'),
    ("ge", '≥'),
    ("ne", '≠'),
    ("asymp", '≈'),
    ("infin", '∞'),
    ("micro", 'µ'),
    ("auml", 'ä'),
    ("ouml", 'ö'),
    ("uuml", 'ü'),
    ("Auml", 'Ä'),
    ("Ouml", 'Ö'),
    ("Uuml", 'Ü'),
    ("szlig", 'ß'),
    ("eacute", 'é'),
    ("egrave", 'è'),
    ("aacute", 'á'),
    ("agrave", 'à'),
    ("ccedil", 'ç'),
    ("ntilde", 'ñ'),
    ("thinsp", '\u{2009}'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("shy", '\u{ad}'),
    ("zwj", '\u{200d}'),
    ("zwnj", '\u{200c}'),
    ("check", '✓'),
    ("minus", '−'),
    ("prime", '′'),
    ("Prime", '″'),
];

/// An entity reference at the start of `text`, and its length.
fn entity(text: &str) -> Option<(Entity, usize)> {
    let rest = text.strip_prefix('&')?;
    let end = rest.find(';')?;
    let name = &rest[..end];
    let used = end + 2;
    if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
            (!hex.is_empty() && hex.len() <= 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| u32::from_str_radix(hex, 16).ok())
                .flatten()?
        } else {
            (!num.is_empty() && num.len() <= 7 && num.chars().all(|c| c.is_ascii_digit()))
                .then(|| num.parse::<u32>().ok())
                .flatten()?
        };
        let ch = if code == 0 {
            '\u{FFFD}'
        } else {
            char::from_u32(code).unwrap_or('\u{FFFD}')
        };
        return Some((Entity::Char(ch), used));
    }
    if name.is_empty()
        || name.len() > 31
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    match ENTITIES.iter().find(|(known, _)| *known == name) {
        Some((_, ch)) => Some((Entity::Char(*ch), used)),
        None => Some((Entity::Raw(format!("&{name};")), used)),
    }
}

/// Text with its known entity references decoded.
pub(crate) fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with('&') {
            if let Some((Entity::Char(ch), used)) = entity(&text[i..]) {
                out.push(ch);
                i += used;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Backslash escapes and entity references resolved, as in a link's
/// destination or a fence's info string.
pub(crate) fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        if ch == '\\' {
            if let Some((_, next)) = chars.peek().copied() {
                if next.is_ascii_punctuation() {
                    out.push(next);
                    chars.next();
                    continue;
                }
            }
        }
        if ch == '&' {
            if let Some((Entity::Char(decoded), used)) = entity(&text[at..]) {
                out.push(decoded);
                for _ in 1..used {
                    chars.next();
                }
                continue;
            }
        }
        out.push(ch);
    }
    out
}

/// The words of nodes, markup dropped: an image's alt text, a title for
/// the search index.
pub(crate) fn plain(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) | Node::Code(text) => out.push_str(text),
            Node::Html(_) | Node::Note(_) => {}
            Node::SoftBreak | Node::HardBreak => out.push(' '),
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) | Node::Link { kids, .. } => {
                out.push_str(&plain(kids));
            }
            Node::Image { alt, .. } => out.push_str(&plain(alt)),
            Node::Math { tex, .. } => out.push_str(tex),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Env, HtmlMode, Node, parse, plain};
    use crate::block::Defs;

    fn nodes(text: &str) -> Vec<Node> {
        let defs = Defs::default();
        parse(
            text,
            &Env {
                defs: &defs,
                html: HtmlMode::Safe,
                math: true,
            },
        )
    }

    #[test]
    fn bare_brackets_stay_text() {
        let got = nodes("Mark [V] and [M] here");
        assert_eq!(plain(&got), "Mark [V] and [M] here");
        assert!(got.iter().all(|n| matches!(n, Node::Text(_))));
    }

    #[test]
    fn a_url_may_hold_parentheses() {
        let got = nodes("[x](https://en.wikipedia.org/wiki/Rust_(language))");
        assert!(
            matches!(&got[0], Node::Link { href, .. } if href == "https://en.wikipedia.org/wiki/Rust_(language)")
        );
    }

    #[test]
    fn bold_italic_nests() {
        let got = nodes("***both***");
        assert!(matches!(&got[0], Node::Em(kids) if matches!(&kids[0], Node::Strong(_))));
    }

    #[test]
    fn money_is_not_maths() {
        let got = nodes("from $5 to $10, and $x^2$");
        assert_eq!(
            got.iter()
                .filter(|n| matches!(n, Node::Math { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn bare_urls_link_without_their_full_stop() {
        let got = nodes("See https://example.com/a. Done");
        assert!(matches!(&got[1], Node::Link { href, .. } if href == "https://example.com/a"));
    }
}
