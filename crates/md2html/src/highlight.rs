// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Code colour without a grammar engine.
//!
//! It is one small lexer that knows each language's comments, strings and
//! keywords, and a few line-shaped formats (diff, YAML, TOML, a shell
//! session). It marks what a reader scans for and leaves the rest as
//! written; a language it does not know is shown plain.

mod lang;
mod lexer;
mod lines;
mod markup;
#[cfg(test)]
mod tests;

use lang::Lang;
use lines::Pairs;

use crate::scan::Scan;
use crate::text::{esc_into, put};

/// The code as highlighted HTML, or None for a language not known here.
pub(crate) fn highlight(lang: &str, code: &str) -> Option<String> {
    let lower = lang.to_ascii_lowercase();
    let name = lower
        .trim_start_matches('{')
        .trim_start_matches('.')
        .trim_end_matches('}');
    Some(match Syntax::named(name)? {
        Syntax::Code(tables) => lexer::lex(code, tables),
        Syntax::Console => lines::console(code),
        Syntax::Diff => lines::diff(code),
        Syntax::Yaml => lines::pairs(code, Pairs::Yaml),
        Syntax::Ini => lines::pairs(code, Pairs::Ini),
        Syntax::Markup => markup::markup(code),
        Syntax::Dockerfile => lines::dockerfile(code),
    })
}

/// How the code of a named language is coloured.
#[derive(Clone, Copy)]
enum Syntax {
    /// By the lexer, with this language's tables.
    Code(&'static Lang),
    /// A shell session: prompts and output.
    Console,
    /// A diff.
    Diff,
    /// YAML.
    Yaml,
    /// TOML, INI and the like.
    Ini,
    /// HTML and XML.
    Markup,
    /// A Dockerfile.
    Dockerfile,
}

impl Syntax {
    /// The syntax a code block's language name stands for, or None for a
    /// name not known here.
    fn named(name: &str) -> Option<Self> {
        let code = match name {
            "rust" | "rs" => &lang::RUST,
            "python" | "py" | "python3" => &lang::PYTHON,
            "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" | "mjs" | "cjs" => {
                &lang::JAVASCRIPT
            }
            "go" | "golang" => &lang::GO,
            "c" | "h" | "cpp" | "c++" | "cc" | "hpp" | "cxx" | "objc" => &lang::C_FAMILY,
            "java" | "kotlin" | "kt" | "swift" | "csharp" | "cs" | "c#" | "scala" | "dart"
            | "groovy" => &lang::CLIKE,
            "sh" | "bash" | "zsh" | "shell" | "fish" | "ksh" | "ps1" | "powershell" => &lang::SHELL,
            "sql" | "sqlite" | "postgres" | "postgresql" | "mysql" | "plsql" => &lang::SQL,
            "ruby" | "rb" => &lang::RUBY,
            "lua" => &lang::LUA,
            "php" => &lang::PHP,
            "css" | "scss" | "less" => &lang::CSS,
            "json" | "jsonc" | "json5" | "jsonl" | "geojson" => &lang::JSON,
            "console" | "terminal" | "shell-session" | "sh-session" => return Some(Self::Console),
            "diff" | "patch" => return Some(Self::Diff),
            "yaml" | "yml" => return Some(Self::Yaml),
            "toml" | "ini" | "cfg" | "conf" | "properties" | "env" | "dotenv" | "editorconfig"
            | "gitconfig" => return Some(Self::Ini),
            "html" | "xml" | "svg" | "xhtml" | "vue" | "plist" => return Some(Self::Markup),
            "dockerfile" | "docker" | "containerfile" => return Some(Self::Dockerfile),
            _ => return None,
        };
        Some(Self::Code(code))
    }
}

/// What a span marks: each is a class the page's style sheet colours.
#[derive(Clone, Copy)]
enum Class {
    /// A keyword.
    Keyword,
    /// A type, a built-in or a variable.
    Type,
    /// A string.
    Str,
    /// A number or a colour.
    Number,
    /// A comment.
    Comment,
    /// A directive, an attribute, an annotation or a prompt.
    Meta,
    /// A key or a property name.
    Attr,
    /// A section heading or a diff's hunk line.
    Heading,
    /// A shell session's output.
    Output,
    /// A line a diff adds.
    Added,
    /// A line a diff removes.
    Removed,
}

impl Class {
    /// The class name in the page.
    const fn name(self) -> &'static str {
        match self {
            Self::Keyword => "k",
            Self::Type => "t",
            Self::Str => "s",
            Self::Number => "n",
            Self::Comment => "c",
            Self::Meta => "m",
            Self::Attr => "a",
            Self::Heading => "h",
            Self::Output => "o",
            Self::Added => "ins",
            Self::Removed => "del",
        }
    }
}

/// One span of a class.
fn span(out: &mut String, class: Class, text: &str) {
    put!(out, "<span class=\"{}\">", class.name());
    esc_into(out, text);
    out.push_str("</span>");
}

/// Reads up to and through `close`, or to the end when it never comes.
fn read_through(scan: &mut Scan<'_>, close: &str) {
    if scan.take_until(close).is_some() {
        scan.eat_str(close);
    } else {
        scan.take_rest();
    }
}
