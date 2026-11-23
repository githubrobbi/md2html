// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The languages the lexer knows, as data: each one's comments, quotes,
//! keyword tables and the syntax features it has beyond those.

use super::Class;

/// A piece of syntax a language may have on top of comments, strings,
/// numbers and words.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Feature {
    /// A capitalised word is a type (Rust, Java, TypeScript...).
    CapsTypes,
    /// Keywords match regardless of case (SQL).
    NoCase,
    /// `#` starts a comment only at the start or after white space (shell).
    HashAfterSpace,
    /// `$name` and `${name}` are variables (shell, PHP).
    DollarVars,
    /// `@name` is an annotation (Python, Java).
    AtMeta,
    /// `#` that only white space precedes on its line is a directive (C).
    HashDirective,
    /// `#[..]` and `#![..]` are attributes (Rust).
    Attributes,
    /// `"""` and `'''` open a string that runs across lines (Python).
    TripleQuotes,
    /// A string followed by `:` is a key (JSON).
    StringKeys,
    /// A style sheet: `name:` after `{` or `;` is a property, `#abc` a
    /// colour, `-` part of a word, `%` the end of a number and `@name` a
    /// rule (CSS).
    StyleSheet,
}

/// The two ends of a block comment.
pub(super) struct Block {
    /// What opens it.
    pub(super) open: &'static str,
    /// What closes it.
    pub(super) close: &'static str,
}

/// The block comment of C and the languages that took it over.
const SLASH_STAR: Option<Block> = Some(Block {
    open: "/*",
    close: "*/",
});

/// What a language's text looks like to the lexer.
pub(super) struct Lang {
    /// Line comments.
    pub(super) line: &'static [&'static str],
    /// The block comment, if it has one.
    pub(super) block: Option<Block>,
    /// Characters that open a string.
    pub(super) quotes: &'static str,
    /// Keywords.
    keywords: &'static [&'static str],
    /// Built-in types, values and functions.
    builtins: &'static [&'static str],
    /// The syntax it has beyond comments, strings and words.
    features: &'static [Feature],
}

impl Lang {
    /// Whether the language has `feature`.
    pub(super) fn has(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    /// The class a word is shown in, or None for a plain word.
    pub(super) fn class_of(&self, word: &str) -> Option<Class> {
        if self.lists(self.keywords, word) {
            Some(Class::Keyword)
        } else if self.lists(self.builtins, word)
            || (self.has(Feature::CapsTypes)
                && word.starts_with(char::is_uppercase)
                && word.chars().any(char::is_lowercase))
        {
            Some(Class::Type)
        } else {
            None
        }
    }

    /// Whether `word` is in `table`, in any case where the language
    /// ignores case (its tables are written in lower case).
    fn lists(&self, table: &[&str], word: &str) -> bool {
        if self.has(Feature::NoCase) {
            table.iter().any(|known| known.eq_ignore_ascii_case(word))
        } else {
            table.contains(&word)
        }
    }

    /// Every word of its tables, for the tests.
    #[cfg(test)]
    pub(super) fn words(&self) -> impl Iterator<Item = &'static str> {
        self.keywords.iter().chain(self.builtins).copied()
    }
}

/// Rust.
pub(super) static RUST: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
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
    features: &[Feature::CapsTypes, Feature::Attributes],
};

/// Python.
pub(super) static PYTHON: Lang = Lang {
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
    features: &[Feature::AtMeta, Feature::TripleQuotes],
};

/// JavaScript and TypeScript.
pub(super) static JAVASCRIPT: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
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
    features: &[Feature::CapsTypes, Feature::AtMeta],
};

/// Go.
pub(super) static GO: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
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
    features: &[],
};

/// C and C++.
pub(super) static C_FAMILY: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
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
    features: &[Feature::HashDirective],
};

/// The C-like family: Java, Kotlin, Swift, C#.
pub(super) static CLIKE: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
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
    features: &[Feature::CapsTypes, Feature::AtMeta],
};

/// The shell.
pub(super) static SHELL: Lang = Lang {
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
    features: &[Feature::HashAfterSpace, Feature::DollarVars],
};

/// SQL.
pub(super) static SQL: Lang = Lang {
    line: &["--"],
    block: SLASH_STAR,
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
    features: &[Feature::NoCase],
};

/// Ruby.
pub(super) static RUBY: Lang = Lang {
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
    features: &[Feature::CapsTypes],
};

/// Lua.
pub(super) static LUA: Lang = Lang {
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
    features: &[],
};

/// PHP.
pub(super) static PHP: Lang = Lang {
    line: &["//", "#"],
    block: SLASH_STAR,
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
    features: &[Feature::CapsTypes, Feature::DollarVars],
};

/// CSS.
pub(super) static CSS: Lang = Lang {
    line: &[],
    block: SLASH_STAR,
    quotes: "\"'",
    keywords: &["important", "inherit", "initial", "none", "auto"],
    builtins: &[],
    features: &[Feature::AtMeta, Feature::StyleSheet],
};

/// JSON.
pub(super) static JSON: Lang = Lang {
    line: &["//"],
    block: SLASH_STAR,
    quotes: "\"",
    keywords: &["true", "false", "null"],
    builtins: &[],
    features: &[Feature::StringKeys],
};
