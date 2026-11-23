// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The `CommonMark` spec's own examples, rendered and compared. The count
//! may rise; it may not fall below `BASELINE`.

use crate::inline::HtmlMode;

/// Examples passing when this was last raised. Raise it when you fix more.
const BASELINE: usize = 640;

/// One example: section, Markdown, expected HTML, number.
struct Example {
    section: String,
    markdown: String,
    html: String,
    number: u64,
}

/// The vendored examples, read with a parser for exactly this JSON.
fn examples() -> Vec<Example> {
    let text = include_str!("../tests/commonmark/spec-0.31.2.json");
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('{') {
        rest = &rest[at + 1..];
        let end = object_end(rest);
        let body = &rest[..end];
        rest = &rest[end..];
        let field = |name: &str| -> Option<String> {
            let key = format!("\"{name}\":");
            let at = body.find(&key)? + key.len();
            let value = body[at..].trim_start();
            value.strip_prefix('"').map_or_else(
                || Some(value.chars().take_while(char::is_ascii_digit).collect()),
                |stripped| Some(json_string(stripped)),
            )
        };
        if let (Some(markdown), Some(html), Some(section), Some(number)) = (
            field("markdown"),
            field("html"),
            field("section"),
            field("example"),
        ) {
            out.push(Example {
                section,
                markdown,
                html,
                number: number.parse().unwrap_or(0),
            });
        }
    }
    out
}

/// Where an object's closing brace is, strings skipped.
fn object_end(text: &str) -> usize {
    let mut in_string = false;
    let mut escaped = false;
    for (at, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == '}' {
            return at;
        }
    }
    text.len()
}

/// A JSON string's value, after its opening quote.
fn json_string(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).unwrap_or(0xFFFD);
                    out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                }
                Some(other) => out.push(other),
                None => break,
            },
            other => out.push(other),
        }
    }
    out
}

/// HTML with the differences that do not matter taken out: self-closing
/// slashes, whitespace between and around tags.
fn normalize(html: &str) -> String {
    let html = html.replace(" />", ">").replace("/>", ">");
    let mut out = String::with_capacity(html.len());
    let mut space = false;
    for ch in html.chars() {
        if ch.is_whitespace() {
            space = true;
            continue;
        }
        if space {
            let after_tag = out.ends_with('>');
            if !(after_tag || ch == '<' || out.is_empty()) {
                out.push(' ');
            }
            space = false;
        }
        out.push(ch);
    }
    out
}

/// The HTML md2html makes of an example.
fn render(markdown: &str) -> String {
    let (blocks, defs) = crate::block::parse(markdown, 1, false, true);
    crate::render::fragment(&blocks, &defs, HtmlMode::Spec, false)
}

#[test]
fn commonmark_spec() {
    let all = examples();
    assert_eq!(all.len(), 652, "the vendored spec has 652 examples");
    let mut by_section: Vec<(String, usize, usize)> = Vec::new();
    let mut passed = 0;
    let mut failed = Vec::new();
    for example in &all {
        let got = std::panic::catch_unwind(|| render(&example.markdown)).unwrap_or_default();
        let ok = normalize(&got) == normalize(&example.html);
        if ok {
            passed += 1;
        } else {
            failed.push(example.number);
        }
        match by_section
            .iter_mut()
            .find(|(s, _, _)| *s == example.section)
        {
            Some((_, p, t)) => {
                *p += usize::from(ok);
                *t += 1;
            }
            None => by_section.push((example.section.clone(), usize::from(ok), 1)),
        }
    }
    for (section, p, t) in &by_section {
        println!("{p:>4}/{t:<4} {section}");
    }
    println!("CommonMark 0.31.2: {passed}/{} examples pass", all.len());
    if std::env::var_os("SPEC_FAILURES").is_some() {
        for number in &failed {
            if let Some(example) = all.iter().find(|e| e.number == *number) {
                println!(
                    "--- example {number} ({})\n{:?}\nwant {:?}\ngot  {:?}",
                    example.section,
                    example.markdown,
                    example.html,
                    render(&example.markdown)
                );
            }
        }
    }
    assert!(
        passed >= BASELINE,
        "conformance fell: {passed} < {BASELINE}"
    );
}
