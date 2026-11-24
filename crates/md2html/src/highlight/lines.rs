// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The formats that are read a line at a time: a shell session, a diff,
//! key and value pairs (YAML, TOML, INI, .env) and a Dockerfile.

use super::lang::SHELL;
use super::{Class, lexer, span};
use crate::scan::Scan;
use crate::text::esc_into;

/// The prompts that start a command line in a shell session.
const PROMPTS: [&str; 4] = ["$ ", "% ", "> ", "# "];

/// The lines that open a diff's header.
const DIFF_HEADERS: [&str; 4] = ["+++", "---", "diff ", "index "];

/// A Dockerfile's instructions.
const INSTRUCTIONS: [&str; 18] = [
    "FROM",
    "RUN",
    "CMD",
    "COPY",
    "ADD",
    "ENV",
    "ARG",
    "WORKDIR",
    "EXPOSE",
    "ENTRYPOINT",
    "USER",
    "VOLUME",
    "LABEL",
    "HEALTHCHECK",
    "SHELL",
    "ONBUILD",
    "STOPSIGNAL",
    "AS",
];

/// The words a value can be that are literals, in lower case.
const LITERALS: [&str; 8] = ["true", "false", "null", "yes", "no", "on", "off", "~"];

/// The two ways of writing keys and values.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pairs {
    /// `key: value`, `- item`, `---` (YAML).
    Yaml,
    /// `key = value` and `[section]` (TOML, INI, .env).
    Ini,
}

impl Pairs {
    /// What stands between a key and its value.
    const fn separator(self) -> char {
        match self {
            Self::Yaml => ':',
            Self::Ini => '=',
        }
    }

    /// What starts a comment.
    const fn comments(self) -> &'static [&'static str] {
        match self {
            Self::Yaml => &["#"],
            Self::Ini => &["#", ";"],
        }
    }
}

/// The code, each line written by `line` (given the line without its
/// line break), the line breaks kept.
fn by_line(code: &str, mut line: impl FnMut(&mut String, &str)) -> String {
    let mut out = String::with_capacity(code.len().saturating_mul(2));
    for text in code.split_inclusive('\n') {
        match text.strip_suffix('\n') {
            Some(body) => {
                line(&mut out, body);
                out.push('\n');
            }
            None => line(&mut out, text),
        }
    }
    out
}

/// A shell session: a prompt line is a command, the rest is output.
pub(super) fn console(code: &str) -> String {
    by_line(code, |out, line| {
        let prompted = PROMPTS
            .iter()
            .find_map(|prompt| Some((*prompt, line.strip_prefix(prompt)?)));
        match prompted {
            Some((prompt, command)) => {
                span(out, Class::Meta, prompt.trim_end());
                out.push(' ');
                out.push_str(&lexer::lex(command, &SHELL));
            }
            None => span(out, Class::Output, line),
        }
    })
}

/// A diff: added, removed and hunk lines.
pub(super) fn diff(code: &str) -> String {
    by_line(code, |out, line| {
        let marked = if DIFF_HEADERS.iter().any(|header| line.starts_with(header)) {
            Some(Class::Meta)
        } else if line.starts_with('+') {
            Some(Class::Added)
        } else if line.starts_with('-') {
            Some(Class::Removed)
        } else if line.starts_with("@@") {
            Some(Class::Heading)
        } else {
            None
        };
        match marked {
            Some(class) => span(out, class, line),
            None => esc_into(out, line),
        }
    })
}

/// A Dockerfile: instructions, comments, and the rest as shell.
pub(super) fn dockerfile(code: &str) -> String {
    by_line(code, |out, line| {
        let mut scan = Scan::new(line);
        let indent = scan.take_while(char::is_whitespace);
        if scan.starts_with("#") {
            span(out, Class::Comment, line);
            return;
        }
        let word = scan.take_while(|ch| !ch.is_whitespace());
        if !word.is_empty()
            && INSTRUCTIONS
                .iter()
                .any(|instruction| instruction.eq_ignore_ascii_case(word))
        {
            out.push_str(indent);
            span(out, Class::Keyword, word);
            out.push_str(&lexer::lex(scan.rest(), &SHELL));
        } else {
            out.push_str(&lexer::lex(line, &SHELL));
        }
    })
}

/// Keys and values written the `style` way.
pub(super) fn pairs(code: &str, style: Pairs) -> String {
    by_line(code, |out, line| pair(out, line, style))
}

/// One line of keys and values: a comment, a document marker, a section,
/// or an optional `- ` and a key and its value.
fn pair(out: &mut String, line: &str, style: Pairs) {
    let mut scan = Scan::new(line);
    out.push_str(scan.take_while(char::is_whitespace));
    let text = scan.rest();
    if style
        .comments()
        .iter()
        .any(|marker| text.starts_with(marker))
    {
        span(out, Class::Comment, text);
    } else if text == "---" || text == "..." {
        span(out, Class::Meta, text);
    } else if style == Pairs::Ini && text.starts_with('[') {
        span(out, Class::Heading, text);
    } else {
        let (dash, item) = text
            .strip_prefix("- ")
            .map_or(("", text), |item| ("- ", item));
        out.push_str(dash);
        match split_key(item, style) {
            Some((key, after)) => {
                span(out, Class::Attr, key);
                out.push(style.separator());
                value(out, after, style);
            }
            None => value(out, item, style),
        }
    }
}

/// The key and what follows its separator, the separator being outside
/// quotes. In YAML a `:` separates only before white space or the end, and
/// a space in a flow collection (`{`, `[`) before any `:` means no key.
fn split_key(text: &str, style: Pairs) -> Option<(&str, &str)> {
    let separator = style.separator();
    let mut scan = Scan::new(text);
    let mut quote = None;
    loop {
        let key = scan.seen();
        let ch = scan.bump()?;
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == separator {
            let glued = scan.peek().is_some_and(|after| !after.is_whitespace());
            if style == Pairs::Ini || !glued {
                return (!key.is_empty()).then_some((key, scan.rest()));
            }
        } else if ch == ' '
            && style == Pairs::Yaml
            && !key.contains(':')
            && key.contains(['{', '['])
        {
            return None;
        }
    }
}

/// A value after a key: a string, a literal, a number or plain, then a
/// trailing comment, the white space around it kept as written.
fn value(out: &mut String, text: &str, style: Pairs) {
    let mut scan = Scan::new(text);
    out.push_str(scan.take_while(char::is_whitespace));
    let length = scan.rest().trim_end().chars().count();
    let core = scan.take_chars(length);
    let (body, comment) = split_comment(core, style.comments()).unwrap_or((core, ""));
    if body.starts_with(['"', '\'']) {
        span(out, Class::Str, body);
    } else if LITERALS
        .iter()
        .any(|literal| literal.eq_ignore_ascii_case(body))
    {
        span(out, Class::Keyword, body);
    } else if is_number(body) {
        span(out, Class::Number, body);
    } else {
        esc_into(out, body);
    }
    if !comment.is_empty() {
        span(out, Class::Comment, comment);
    }
    out.push_str(scan.rest());
}

/// A value and its trailing comment, which starts at the first space
/// before a comment marker; None without one, or with a quote before it.
fn split_comment<'text>(text: &'text str, markers: &[&str]) -> Option<(&'text str, &'text str)> {
    let mut scan = Scan::new(text);
    while !scan.is_done() && !opens_comment(scan.rest(), markers) {
        scan.bump();
    }
    let body = scan.seen();
    (!scan.is_done() && !body.contains(['"', '\''])).then_some((body, scan.rest()))
}

/// Whether `text` starts with a space and a comment marker.
fn opens_comment(text: &str, markers: &[&str]) -> bool {
    text.strip_prefix(' ')
        .is_some_and(|after| markers.iter().any(|marker| after.starts_with(marker)))
}

/// Whether a value is a number (or a date or a time): digits and the
/// signs, points and colons they are written with, from a digit or `-`.
fn is_number(text: &str) -> bool {
    text.starts_with(|ch: char| ch.is_ascii_digit() || ch == '-')
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | '_' | 'e' | 'E' | ':'))
}
