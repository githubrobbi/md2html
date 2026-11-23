// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The `CommonMark` spec's own examples, rendered and compared. The count
//! may rise; it may not fall below `BASELINE`.
//!
//! With `SPEC_FAILURES` set the test asks for every example to pass, and
//! so fails with each one that does not, its Markdown, the HTML it should
//! make and the HTML md2html makes.

use crate::inline::HtmlMode;
use crate::scan::Scan;
use crate::text::put;

/// Examples passing when this was last raised. Raise it when you fix more.
const BASELINE: usize = 652;

/// How many examples the vendored spec has.
const EXAMPLES: usize = 652;

/// One example of the spec.
struct Example {
    /// The section it belongs to.
    section: String,
    /// The Markdown.
    markdown: String,
    /// The HTML it makes.
    html: String,
    /// Its number in the spec.
    number: u64,
}

/// A value of the spec's JSON: a string or a whole number.
enum Value {
    /// A string, unescaped.
    Text(String),
    /// A whole number.
    Number(u64),
}

/// An example's fields as they are read, in any order.
#[derive(Default)]
struct Fields {
    /// `section`.
    section: Option<String>,
    /// `markdown`.
    markdown: Option<String>,
    /// `html`.
    html: Option<String>,
    /// `example`.
    number: Option<u64>,
}

impl Fields {
    /// Keeps a field this test needs; the others (`start_line`,
    /// `end_line`) are passed over.
    fn set(&mut self, key: &str, value: Value) {
        match (key, value) {
            ("section", Value::Text(text)) => self.section = Some(text),
            ("markdown", Value::Text(text)) => self.markdown = Some(text),
            ("html", Value::Text(text)) => self.html = Some(text),
            ("example", Value::Number(number)) => self.number = Some(number),
            (_, Value::Text(_) | Value::Number(_)) => {}
        }
    }

    /// The example, when every field it needs was there.
    fn example(self) -> Option<Example> {
        Some(Example {
            section: self.section?,
            markdown: self.markdown?,
            html: self.html?,
            number: self.number?,
        })
    }
}

/// A reader of the spec's JSON: an array of flat objects whose values are
/// strings and whole numbers.
struct Json<'text> {
    /// The text, read from the front.
    scan: Scan<'text>,
}

impl Json<'_> {
    /// Reads past spaces and line breaks.
    fn skip_space(&mut self) {
        self.scan.take_while(char::is_whitespace);
    }

    /// Reads `ch`, spaces before it skipped; says whether it was there.
    fn eat(&mut self, ch: char) -> bool {
        self.skip_space();
        self.scan.eat(ch)
    }

    /// Every example in the array.
    fn examples(&mut self) -> Vec<Example> {
        let mut out = Vec::new();
        if !self.eat('[') {
            return out;
        }
        while self.eat('{') {
            out.extend(self.object().and_then(Fields::example));
            if !self.eat(',') {
                break;
            }
        }
        out
    }

    /// The fields of an object, after its `{`, to its `}`.
    fn object(&mut self) -> Option<Fields> {
        let mut fields = Fields::default();
        if self.eat('}') {
            return Some(fields);
        }
        loop {
            let key = self.string()?;
            if !self.eat(':') {
                return None;
            }
            let value = self.value()?;
            fields.set(&key, value);
            if self.eat('}') {
                return Some(fields);
            }
            if !self.eat(',') {
                return None;
            }
        }
    }

    /// A string or a whole number.
    fn value(&mut self) -> Option<Value> {
        self.skip_space();
        if self.scan.starts_with("\"") {
            return self.string().map(Value::Text);
        }
        let digits = self.scan.take_while(|ch| ch.is_ascii_digit());
        digits.parse().ok().map(Value::Number)
    }

    /// A string, quotes and all, its escapes undone.
    fn string(&mut self) -> Option<String> {
        if !self.eat('"') {
            return None;
        }
        let mut out = String::new();
        loop {
            match self.scan.bump()? {
                '"' => return Some(out),
                '\\' => out.push(self.escaped()?),
                other => out.push(other),
            }
        }
    }

    /// The character an escape after a backslash stands for.
    fn escaped(&mut self) -> Option<char> {
        Some(match self.scan.bump()? {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'u' => {
                let code = u32::from_str_radix(self.scan.take_chars(4), 16).ok()?;
                char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            other => other,
        })
    }
}

/// The vendored examples.
fn examples() -> Vec<Example> {
    let text = include_str!("../tests/commonmark/spec-0.31.2.json");
    Json {
        scan: Scan::new(text),
    }
    .examples()
}

/// HTML with the differences that do not matter taken out: self-closing
/// slashes, whitespace between and around tags.
fn normalize(html: &str) -> String {
    let unslashed = html.replace(" />", ">").replace("/>", ">");
    let mut out = String::with_capacity(unslashed.len());
    let mut space = false;
    for ch in unslashed.chars() {
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

/// The HTML md2html makes of an example; empty if rendering it failed.
fn render(markdown: &str) -> String {
    std::panic::catch_unwind(|| {
        let (blocks, defs) =
            crate::block::parse(markdown, 1, false, crate::block::Reading::CommonMark);
        crate::render::fragment(&blocks, &defs, HtmlMode::Spec, false)
    })
    .unwrap_or_default()
}

/// An example that md2html renders otherwise than the spec.
struct Failure<'spec> {
    /// The example.
    example: &'spec Example,
    /// What md2html made of it.
    got: String,
}

/// The failures, one after another, their text with escapes shown.
fn listing(failures: &[Failure<'_>]) -> String {
    let mut out = String::new();
    for Failure { example, got } in failures {
        put!(
            out,
            "\n--- example {} ({})\n{}\nwant {}\ngot  {}\n",
            example.number,
            example.section,
            example.markdown.escape_debug(),
            example.html.escape_debug(),
            got.escape_debug()
        );
    }
    out
}

/// The examples of the CommonMark specification render as it says.
///
/// # Panics
///
/// When the vendored specification does not hold its [`EXAMPLES`] examples,
/// or when fewer than [`BASELINE`] of them render to the HTML the
/// specification gives (compared after [`normalize`]); the message lists
/// the numbers of those that fail. With `SPEC_FAILURES` set, also when any
/// example fails, each shown with its Markdown, the HTML it should be and
/// the HTML it is.
#[test]
fn commonmark_spec() {
    let all = examples();
    assert_eq!(
        all.len(),
        EXAMPLES,
        "the vendored spec has {EXAMPLES} examples"
    );
    let failures: Vec<Failure<'_>> = all
        .iter()
        .filter_map(|example| {
            let got = render(&example.markdown);
            (normalize(&got) != normalize(&example.html)).then_some(Failure { example, got })
        })
        .collect();
    let passed = all.len().saturating_sub(failures.len());
    let numbers: Vec<String> = failures
        .iter()
        .map(|failure| failure.example.number.to_string())
        .collect();
    assert!(
        passed >= BASELINE,
        "conformance fell: {passed} of {EXAMPLES} < {BASELINE}; failing: {}",
        numbers.join(", ")
    );
    if std::env::var_os("SPEC_FAILURES").is_some() {
        assert!(
            failures.is_empty(),
            "CommonMark 0.31.2: {passed}/{EXAMPLES} examples pass{}",
            listing(&failures)
        );
    }
}
