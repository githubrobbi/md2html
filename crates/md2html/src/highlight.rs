// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Code colour without a grammar engine: one small lexer that knows each
//! language's comments, strings and keywords, and a few line-shaped
//! formats (diff, YAML, TOML, a shell session). It marks what a reader
//! scans for and leaves the rest as written; a language it does not know
//! is shown plain.
//!
//! Exception: mostly data: the keyword tables of some twenty languages
//! (`scripts/ci/file_size_exceptions.txt`).

use crate::text::esc;

/// What a language's text looks like to the lexer.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent settings, each a flag of its own"
)]
struct Lang {
    /// Line comments.
    line: &'static [&'static str],
    /// Block comments.
    block: Option<(&'static str, &'static str)>,
    /// Characters that open a string.
    quotes: &'static str,
    /// Keywords.
    keywords: &'static [&'static str],
    /// Built-in types, values and functions.
    builtins: &'static [&'static str],
    /// A capitalised word is a type (Rust, Go, Java, TypeScript...).
    caps_types: bool,
    /// Keywords match regardless of case (SQL).
    nocase: bool,
    /// `#` starts a comment only at a word's start (shell).
    hash_word: bool,
    /// `$name` is a variable (shell, PHP).
    dollar_vars: bool,
    /// `@name` is an annotation (Python, Java).
    at_meta: bool,
    /// `#` at a line's start is a directive (C).
    hash_directive: bool,
}

/// Rust.
static RUST: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"",
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "static", "struct", "super", "trait", "type", "unsafe", "use",
        "where", "while", "yield", "self", "Self",
    ],
    builtins: &[
        "true", "false", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize", "f32", "f64", "bool", "char", "str", "Some", "None", "Ok", "Err",
    ],
    caps_types: true,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// Python.
static PYTHON: Lang = Lang {
    line: &["#"],
    block: None,
    quotes: "\"'",
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
        "yield", "match", "case",
    ],
    builtins: &[
        "None",
        "True",
        "False",
        "self",
        "cls",
        "print",
        "len",
        "range",
        "int",
        "str",
        "float",
        "list",
        "dict",
        "set",
        "tuple",
        "open",
        "bool",
        "bytes",
        "isinstance",
        "super",
        "enumerate",
        "zip",
        "sorted",
        "min",
        "max",
        "sum",
    ],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: true,
    hash_directive: false,
};

/// JavaScript and TypeScript.
static JS: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"'`",
    keywords: &[
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "of",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        "async",
        "await",
        "static",
        "get",
        "set",
        "interface",
        "type",
        "enum",
        "implements",
        "private",
        "public",
        "protected",
        "readonly",
        "declare",
        "namespace",
        "abstract",
        "as",
        "from",
        "keyof",
        "satisfies",
    ],
    builtins: &[
        "true",
        "false",
        "null",
        "undefined",
        "NaN",
        "Infinity",
        "string",
        "number",
        "boolean",
        "any",
        "unknown",
        "never",
        "console",
        "window",
        "document",
        "JSON",
        "Math",
        "Promise",
        "Array",
        "Object",
    ],
    caps_types: true,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: true,
    hash_directive: false,
};

/// Go.
static GO: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"`'",
    keywords: &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "type",
        "var",
    ],
    builtins: &[
        "bool",
        "byte",
        "complex64",
        "complex128",
        "error",
        "float32",
        "float64",
        "int",
        "int8",
        "int16",
        "int32",
        "int64",
        "rune",
        "string",
        "uint",
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "uintptr",
        "true",
        "false",
        "nil",
        "iota",
        "len",
        "cap",
        "make",
        "new",
        "append",
        "panic",
    ],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// C and C++.
static C: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"'",
    keywords: &[
        "auto",
        "break",
        "case",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extern",
        "for",
        "goto",
        "if",
        "inline",
        "register",
        "restrict",
        "return",
        "sizeof",
        "static",
        "struct",
        "switch",
        "typedef",
        "union",
        "volatile",
        "while",
        "class",
        "namespace",
        "template",
        "typename",
        "public",
        "private",
        "protected",
        "virtual",
        "override",
        "new",
        "delete",
        "this",
        "using",
        "try",
        "catch",
        "throw",
        "constexpr",
        "noexcept",
        "operator",
        "friend",
        "explicit",
        "mutable",
    ],
    builtins: &[
        "char", "double", "float", "int", "long", "short", "signed", "unsigned", "void", "bool",
        "true", "false", "NULL", "nullptr", "size_t", "uint8_t", "uint16_t", "uint32_t",
        "uint64_t", "int8_t", "int16_t", "int32_t", "int64_t", "std", "string", "vector", "auto",
    ],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: true,
};

/// The C-like family: Java, Kotlin, Swift, C#.
static CLIKE: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"'",
    keywords: &[
        "abstract",
        "assert",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "for",
        "if",
        "implements",
        "import",
        "instanceof",
        "interface",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "static",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "volatile",
        "while",
        "var",
        "val",
        "fun",
        "func",
        "let",
        "guard",
        "struct",
        "protocol",
        "extension",
        "when",
        "object",
        "data",
        "sealed",
        "override",
        "open",
        "internal",
        "namespace",
        "using",
        "async",
        "await",
        "record",
        "in",
        "is",
        "as",
        "where",
        "init",
        "self",
        "def",
        "lazy",
        "match",
        "trait",
        "yield",
    ],
    builtins: &[
        "true", "false", "null", "nil", "boolean", "byte", "char", "double", "float", "int",
        "long", "short", "void", "string", "String", "Int", "Bool", "Double", "Unit", "Any",
    ],
    caps_types: true,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: true,
    hash_directive: false,
};

/// The shell.
static SHELL: Lang = Lang {
    line: &["#"],
    block: None,
    quotes: "\"'",
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "in", "function", "return", "local", "export", "source", "alias", "set", "unset", "exit",
        "readonly", "declare", "shift", "trap",
    ],
    builtins: &[
        "echo", "cd", "printf", "read", "test", "true", "false", "eval", "exec", "sudo", "cargo",
        "git", "curl", "cat", "grep", "sed", "awk", "ls", "mkdir", "rm", "cp", "mv", "brew", "npm",
        "npx", "python3", "pip", "just",
    ],
    caps_types: false,
    nocase: false,
    hash_word: true,
    dollar_vars: true,
    at_meta: false,
    hash_directive: false,
};

/// SQL.
static SQL: Lang = Lang {
    line: &["--"],
    block: Some(("/*", "*/")),
    quotes: "'\"",
    keywords: &[
        "select",
        "from",
        "where",
        "and",
        "or",
        "not",
        "insert",
        "into",
        "values",
        "update",
        "set",
        "delete",
        "create",
        "table",
        "index",
        "view",
        "drop",
        "alter",
        "add",
        "join",
        "left",
        "right",
        "inner",
        "outer",
        "full",
        "on",
        "group",
        "by",
        "order",
        "having",
        "limit",
        "offset",
        "as",
        "distinct",
        "union",
        "all",
        "case",
        "when",
        "then",
        "else",
        "end",
        "is",
        "like",
        "in",
        "between",
        "exists",
        "primary",
        "key",
        "foreign",
        "references",
        "default",
        "unique",
        "begin",
        "commit",
        "rollback",
        "with",
        "returning",
        "asc",
        "desc",
        "if",
        "pragma",
        "trigger",
    ],
    builtins: &[
        "null",
        "true",
        "false",
        "integer",
        "int",
        "text",
        "varchar",
        "real",
        "blob",
        "boolean",
        "date",
        "timestamp",
        "count",
        "sum",
        "avg",
        "min",
        "max",
        "coalesce",
    ],
    caps_types: false,
    nocase: true,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// Ruby.
static RUBY: Lang = Lang {
    line: &["#"],
    block: None,
    quotes: "\"'",
    keywords: &[
        "def",
        "end",
        "class",
        "module",
        "if",
        "elsif",
        "else",
        "unless",
        "while",
        "until",
        "for",
        "do",
        "return",
        "yield",
        "require",
        "include",
        "begin",
        "rescue",
        "ensure",
        "case",
        "when",
        "then",
        "and",
        "or",
        "not",
        "in",
        "attr_reader",
        "attr_accessor",
        "local",
        "function",
    ],
    builtins: &["nil", "true", "false", "self", "puts", "print"],
    caps_types: true,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// Lua.
static LUA: Lang = Lang {
    line: &["--"],
    block: None,
    quotes: "\"'",
    keywords: &[
        "and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in",
        "local", "not", "or", "repeat", "return", "then", "until", "while",
    ],
    builtins: &[
        "nil", "true", "false", "print", "pairs", "ipairs", "require",
    ],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// PHP.
static PHP: Lang = Lang {
    line: &["//", "#"],
    block: Some(("/*", "*/")),
    quotes: "\"'",
    keywords: &[
        "function",
        "class",
        "public",
        "private",
        "protected",
        "static",
        "return",
        "if",
        "else",
        "elseif",
        "foreach",
        "for",
        "while",
        "as",
        "new",
        "use",
        "namespace",
        "echo",
        "array",
        "fn",
        "match",
        "try",
        "catch",
        "throw",
    ],
    builtins: &["true", "false", "null", "this"],
    caps_types: true,
    nocase: false,
    hash_word: false,
    dollar_vars: true,
    at_meta: false,
    hash_directive: false,
};

/// CSS.
static CSS: Lang = Lang {
    line: &[],
    block: Some(("/*", "*/")),
    quotes: "\"'",
    keywords: &["important", "inherit", "initial", "none", "auto"],
    builtins: &[],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: true,
    hash_directive: false,
};

/// JSON.
static JSON: Lang = Lang {
    line: &["//"],
    block: Some(("/*", "*/")),
    quotes: "\"",
    keywords: &["true", "false", "null"],
    builtins: &[],
    caps_types: false,
    nocase: false,
    hash_word: false,
    dollar_vars: false,
    at_meta: false,
    hash_directive: false,
};

/// The code as highlighted HTML, or None for a language not known here.
pub(crate) fn highlight(lang: &str, code: &str) -> Option<String> {
    let lang = lang.to_ascii_lowercase();
    let lang = lang
        .trim_start_matches('{')
        .trim_start_matches('.')
        .trim_end_matches('}');
    let spec = match lang {
        "rust" | "rs" => &RUST,
        "python" | "py" | "python3" => &PYTHON,
        "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" | "mjs" | "cjs" => &JS,
        "go" | "golang" => &GO,
        "c" | "h" | "cpp" | "c++" | "cc" | "hpp" | "cxx" | "objc" => &C,
        "java" | "kotlin" | "kt" | "swift" | "csharp" | "cs" | "c#" | "scala" | "dart"
        | "groovy" => &CLIKE,
        "sh" | "bash" | "zsh" | "shell" | "fish" | "ksh" | "ps1" | "powershell" => &SHELL,
        "sql" | "sqlite" | "postgres" | "postgresql" | "mysql" | "plsql" => &SQL,
        "ruby" | "rb" => &RUBY,
        "lua" => &LUA,
        "php" => &PHP,
        "css" | "scss" | "less" => &CSS,
        "json" | "jsonc" | "json5" | "jsonl" | "geojson" => {
            return Some(lex(code, &JSON, Mode::Json));
        }
        "console" | "terminal" | "shell-session" | "sh-session" => return Some(console(code)),
        "diff" | "patch" => return Some(diff(code)),
        "yaml" | "yml" => return Some(keyed(code, ':', &["#"])),
        "toml" | "ini" | "cfg" | "conf" | "properties" | "env" | "dotenv" | "editorconfig"
        | "gitconfig" => {
            return Some(keyed(code, '=', &["#", ";"]));
        }
        "html" | "xml" | "svg" | "xhtml" | "vue" | "plist" => return Some(markup(code)),
        "dockerfile" | "docker" | "containerfile" => return Some(dockerfile(code)),
        _ => return None,
    };
    let mode = if core::ptr::eq(spec, &raw const CSS) {
        Mode::Css
    } else {
        Mode::Code
    };
    Some(lex(code, spec, mode))
}

/// Special rules on top of the generic lexer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// None.
    Code,
    /// A string followed by `:` is a key.
    Json,
    /// `name:` is a property, `#abc` a colour, `@media` a rule.
    Css,
}

/// One span of a class.
fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&esc(text));
    out.push_str("</span>");
}

/// The generic lexer.
#[expect(
    clippy::too_many_lines,
    reason = "one dispatch over the kinds it handles, read top to bottom"
)]
fn lex(code: &str, lang: &Lang, mode: Mode) -> String {
    let chars: Vec<char> = code.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(code.len() * 2);
    let mut i = 0;
    let at = |i: usize, s: &str| {
        s.chars()
            .enumerate()
            .all(|(k, c)| chars.get(i + k) == Some(&c))
    };
    let take = |from: usize, to: usize| chars[from..to.min(n)].iter().collect::<String>();
    let word_char = |c: char| c.is_alphanumeric() || c == '_';
    while i < n {
        let c = chars[i];
        let line_start = i == 0
            || chars[i - 1] == '\n'
            || chars[..i]
                .iter()
                .rev()
                .take_while(|c| **c != '\n')
                .all(|c| c.is_whitespace());
        if lang.hash_directive && c == '#' && line_start {
            let end = chars[i..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(n, |p| i + p);
            span(&mut out, "m", &take(i, end));
            i = end;
            continue;
        }
        if let Some((open, close)) = lang.block {
            if at(i, open) {
                let start = i;
                i += open.chars().count();
                while i < n && !at(i, close) {
                    i += 1;
                }
                i = (i + close.chars().count()).min(n);
                span(&mut out, "c", &take(start, i));
                continue;
            }
        }
        if let Some(marker) = lang.line.iter().find(|m| at(i, m)) {
            let ok = !lang.hash_word || *marker != "#" || i == 0 || chars[i - 1].is_whitespace();
            if ok {
                let end = chars[i..]
                    .iter()
                    .position(|c| *c == '\n')
                    .map_or(n, |p| i + p);
                span(&mut out, "c", &take(i, end));
                i = end;
                continue;
            }
        }
        if lang.quotes.contains(c) {
            // A Rust lifetime ('a) is not a string: a char literal closes
            // within a few characters.
            if c == '\'' && core::ptr::eq(lang, &raw const RUST) {
                let close = chars[i + 1..n.min(i + 5)].iter().position(|x| *x == '\'');
                if close.is_none() {
                    out.push('\'');
                    i += 1;
                    continue;
                }
            }
            let triple = (c == '"' || c == '\'')
                && at(i, &core::iter::repeat_n(c, 3).collect::<String>())
                && core::ptr::eq(lang, &raw const PYTHON);
            let start = i;
            i += if triple { 3 } else { 1 };
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if triple {
                    if at(i, &core::iter::repeat_n(c, 3).collect::<String>()) {
                        i += 3;
                        break;
                    }
                } else if chars[i] == c {
                    i += 1;
                    break;
                } else if chars[i] == '\n' && c != '`' {
                    break;
                }
                i += 1;
            }
            let text = take(start, i);
            let mut j = i;
            while j < n && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }
            let key = mode == Mode::Json && chars.get(j) == Some(&':');
            span(&mut out, if key { "a" } else { "s" }, &text);
            continue;
        }
        if lang.dollar_vars
            && c == '$'
            && i + 1 < n
            && (word_char(chars[i + 1]) || chars[i + 1] == '{')
        {
            let start = i;
            i += 1;
            if chars[i] == '{' {
                while i < n && chars[i] != '}' {
                    i += 1;
                }
                i = (i + 1).min(n);
            } else {
                while i < n && word_char(chars[i]) {
                    i += 1;
                }
            }
            span(&mut out, "t", &take(start, i));
            continue;
        }
        if lang.at_meta
            && c == '@'
            && i + 1 < n
            && chars[i + 1].is_alphabetic()
            && (i == 0 || !word_char(chars[i - 1]))
        {
            let start = i;
            i += 1;
            while i < n && (word_char(chars[i]) || chars[i] == '.' || chars[i] == '-') {
                i += 1;
            }
            span(
                &mut out,
                if mode == Mode::Css { "k" } else { "m" },
                &take(start, i),
            );
            continue;
        }
        if core::ptr::eq(lang, &raw const RUST)
            && c == '#'
            && chars.get(i + 1).is_some_and(|x| *x == '[' || *x == '!')
        {
            let start = i;
            let mut depth = 0;
            while i < n {
                if chars[i] == '[' {
                    depth += 1;
                } else if chars[i] == ']' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                } else if chars[i] == '\n' {
                    break;
                }
                i += 1;
            }
            span(&mut out, "m", &take(start, i));
            continue;
        }
        if mode == Mode::Css && c == '#' && chars.get(i + 1).is_some_and(char::is_ascii_hexdigit) {
            let start = i;
            i += 1;
            while i < n && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            span(&mut out, "n", &take(start, i));
            continue;
        }
        if c.is_ascii_digit() && (i == 0 || !word_char(chars[i - 1])) {
            let start = i;
            while i < n
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_'
                    || (chars[i] == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)))
            {
                i += 1;
            }
            if mode == Mode::Css {
                while i < n && (chars[i] == '%') {
                    i += 1;
                }
            }
            span(&mut out, "n", &take(start, i));
            continue;
        }
        if c.is_alphabetic() || c == '_' || (mode == Mode::Css && c == '-') {
            let start = i;
            while i < n && (word_char(chars[i]) || (mode == Mode::Css && chars[i] == '-')) {
                i += 1;
            }
            let word = take(start, i);
            let lower = word.to_ascii_lowercase();
            let probe = if lang.nocase {
                lower.as_str()
            } else {
                word.as_str()
            };
            if mode == Mode::Css {
                let mut j = i;
                while j < n && chars[j] == ' ' {
                    j += 1;
                }
                if chars.get(j) == Some(&':')
                    && chars[..start]
                        .iter()
                        .rev()
                        .find(|c| !c.is_whitespace())
                        .is_some_and(|p| *p == '{' || *p == ';')
                {
                    span(&mut out, "a", &word);
                    continue;
                }
            }
            if lang.keywords.contains(&probe) {
                span(&mut out, "k", &word);
            } else if lang.builtins.contains(&probe)
                || (lang.caps_types
                    && word.starts_with(|c: char| c.is_uppercase())
                    && word.chars().any(char::is_lowercase))
            {
                span(&mut out, "t", &word);
            } else {
                out.push_str(&esc(&word));
            }
            continue;
        }
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
        i += 1;
    }
    out
}

/// A shell session: a prompt line is a command, the rest is output.
fn console(code: &str) -> String {
    let mut out = String::new();
    for line in code.split_inclusive('\n') {
        let (body, nl) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let prompt = ["$ ", "% ", "> ", "# "]
            .iter()
            .find(|p| body.starts_with(**p));
        match prompt {
            Some(p) => {
                span(&mut out, "m", p.trim_end());
                out.push(' ');
                out.push_str(&lex(&body[p.len()..], &SHELL, Mode::Code));
            }
            None => span(&mut out, "o", body),
        }
        out.push_str(nl);
    }
    out
}

/// A diff: added, removed and hunk lines.
fn diff(code: &str) -> String {
    let mut out = String::new();
    for line in code.split_inclusive('\n') {
        let (body, nl) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let class = if body.starts_with("+++")
            || body.starts_with("---")
            || body.starts_with("diff ")
            || body.starts_with("index ")
        {
            "m"
        } else if body.starts_with('+') {
            "ins"
        } else if body.starts_with('-') {
            "del"
        } else if body.starts_with("@@") {
            "h"
        } else {
            ""
        };
        if class.is_empty() {
            out.push_str(&esc(body));
        } else {
            span(&mut out, class, body);
        }
        out.push_str(nl);
    }
    out
}

/// `key: value` and `key = value` formats (YAML, TOML, INI, .env).
fn keyed(code: &str, sep: char, comments: &[&str]) -> String {
    let mut out = String::new();
    for line in code.split_inclusive('\n') {
        let (body, nl) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let trimmed = body.trim_start();
        let lead = &body[..body.len() - trimmed.len()];
        out.push_str(lead);
        if comments.iter().any(|c| trimmed.starts_with(c)) {
            span(&mut out, "c", trimmed);
        } else if trimmed == "---" || trimmed == "..." {
            span(&mut out, "m", trimmed);
        } else if sep == '=' && trimmed.starts_with('[') {
            span(&mut out, "h", trimmed);
        } else {
            let (dash, rest) = trimmed
                .strip_prefix("- ")
                .map_or(("", trimmed), |rest| ("- ", rest));
            out.push_str(dash);
            let split = find_sep(rest, sep);
            match split {
                Some(at) => {
                    span(&mut out, "a", &rest[..at]);
                    out.push_str(&esc(&rest[at..=at]));
                    value(&mut out, &rest[at + 1..], comments);
                }
                None => value(&mut out, rest, comments),
            }
        }
        out.push_str(nl);
    }
    out
}

/// Where a key's separator is, outside quotes.
fn find_sep(text: &str, sep: char) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (at, ch) in text.char_indices() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                }
            }
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == sep => {
                let after = text[at + ch.len_utf8()..].chars().next();
                if sep == ':' && after.is_some_and(|c| !c.is_whitespace()) {
                    continue;
                }
                return (at > 0).then_some(at);
            }
            None if ch == ' '
                && sep == ':'
                && !text[..at].contains(':')
                && text[..at].contains(['{', '[']) =>
            {
                return None;
            }
            None => {}
        }
    }
    None
}

/// A value after a key: strings, numbers, literals, a trailing comment.
fn value(out: &mut String, text: &str, comments: &[&str]) {
    let trimmed = text.trim();
    let lead = &text[..text.len() - text.trim_start().len()];
    out.push_str(lead);
    let comment_at = comments
        .iter()
        .filter_map(|c| trimmed.find(&format!(" {c}")))
        .min()
        .filter(|at| !trimmed[..*at].contains(['"', '\'']));
    let (body, comment) =
        comment_at.map_or((trimmed, None), |at| (&trimmed[..at], Some(&trimmed[at..])));
    let lower = body.to_ascii_lowercase();
    if body.starts_with(['"', '\'']) {
        span(out, "s", body);
    } else if matches!(
        lower.as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
    ) {
        span(out, "k", body);
    } else if !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | '_' | 'e' | 'E' | ':'))
        && body
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '-')
    {
        span(out, "n", body);
    } else {
        out.push_str(&esc(body));
    }
    if let Some(comment) = comment {
        span(out, "c", comment);
    }
    out.push_str(&text[lead.len() + trimmed.len()..]);
}

/// HTML and XML: tags, attributes, strings, comments.
fn markup(code: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < code.len() {
        let rest = &code[i..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map_or(rest.len(), |e| e + 3);
            span(&mut out, "c", &rest[..end]);
            i += end;
            continue;
        }
        if rest.starts_with('<') {
            let end = rest.find('>').map_or(rest.len(), |e| e + 1);
            let tag = &rest[..end];
            let name_len = tag
                .char_indices()
                .skip(1)
                .find(|(_, c)| c.is_whitespace() || *c == '>')
                .map_or(tag.len(), |(at, _)| at);
            span(&mut out, "k", &tag[..name_len]);
            let mut attrs = &tag[name_len..];
            while !attrs.is_empty() {
                if let Some(q) = attrs.find(['"', '\'']) {
                    let quote = attrs[q..].chars().next().unwrap_or('"');
                    let close = attrs[q + 1..]
                        .find(quote)
                        .map_or(attrs.len(), |c| q + 1 + c + 1);
                    let before = &attrs[..q];
                    match before.rfind('=') {
                        Some(eq) => {
                            let name_start =
                                before[..eq].rfind(char::is_whitespace).map_or(0, |s| s + 1);
                            out.push_str(&esc(&before[..name_start]));
                            span(&mut out, "a", &before[name_start..eq]);
                            out.push('=');
                            out.push_str(&esc(&before[eq + 1..]));
                        }
                        None => out.push_str(&esc(before)),
                    }
                    span(&mut out, "s", &attrs[q..close]);
                    attrs = &attrs[close..];
                } else {
                    out.push_str(&esc(attrs));
                    attrs = "";
                }
            }
            i += end;
            continue;
        }
        let next = rest.find('<').unwrap_or(rest.len());
        out.push_str(&esc(&rest[..next]));
        i += next;
    }
    out
}

/// A Dockerfile: instructions, comments, strings.
fn dockerfile(code: &str) -> String {
    const WORDS: &[&str] = &[
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
    let mut out = String::new();
    for line in code.split_inclusive('\n') {
        let (body, nl) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let trimmed = body.trim_start();
        if trimmed.starts_with('#') {
            span(&mut out, "c", body);
        } else {
            let word_len = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            if WORDS.contains(&trimmed[..word_len].to_ascii_uppercase().as_str()) && word_len > 0 {
                out.push_str(&body[..body.len() - trimmed.len()]);
                span(&mut out, "k", &trimmed[..word_len]);
                out.push_str(&lex(&trimmed[word_len..], &SHELL, Mode::Code));
            } else {
                out.push_str(&lex(body, &SHELL, Mode::Code));
            }
        }
        out.push_str(nl);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn rust_keywords_strings_and_comments() {
        let html = highlight("rust", "fn main() { let s = \"hi\"; // greet\n}").unwrap_or_default();
        assert!(html.contains("<span class=\"k\">fn</span>"), "{html}");
        assert!(
            html.contains("<span class=\"s\">&quot;hi&quot;</span>"),
            "{html}"
        );
        assert!(html.contains("<span class=\"c\">// greet</span>"), "{html}");
    }

    #[test]
    fn a_lifetime_is_not_a_string() {
        let html = highlight("rust", "fn f<'a>(x: &'a str) -> &'a str { x }").unwrap_or_default();
        assert!(!html.contains("class=\"s\""), "{html}");
    }

    #[test]
    fn json_keys_and_values() {
        let html = highlight("json", "{\"a\": 1, \"b\": true}").unwrap_or_default();
        assert!(
            html.contains("<span class=\"a\">&quot;a&quot;</span>"),
            "{html}"
        );
        assert!(html.contains("<span class=\"k\">true</span>"), "{html}");
    }

    #[test]
    fn unknown_languages_are_plain() {
        assert!(highlight("brainfuck", "+++").is_none());
    }

    #[test]
    fn the_text_survives_highlighting() {
        for (lang, code) in [
            ("sh", "echo \"$HOME\" # x <y>\n"),
            ("yaml", "a: b # c\n- d: 1\n"),
            ("html", "<a href=\"x\">y</a>"),
            ("diff", "+a\n-b\n"),
        ] {
            let html = highlight(lang, code).unwrap_or_default();
            assert_eq!(
                crate::text::plain(&html.replace('\n', " ")).replace(' ', ""),
                code.replace(['\n', ' '], ""),
                "{lang}"
            );
        }
    }
}
