// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the highlighter marks, language by language.

use super::highlight;
use super::lang::{Feature, SQL};

/// The code highlighted as `lang`, which must be known.
///
/// # Panics
///
/// Panics when the highlighter does not know `lang` and leaves the code plain.
fn lit(lang: &str, code: &str) -> String {
    highlight(lang, code).expect("a known language")
}

/// Rust code has its keywords, strings and line comments marked.
///
/// # Panics
///
/// Panics when `fn` is not marked a keyword, `"hi"` not a string (its quotes
/// escaped), or `// greet` not a comment.
#[test]
fn rust_keywords_strings_and_comments() {
    let html = lit("rust", "fn main() { let s = \"hi\"; // greet\n}");
    assert!(html.contains("<span class=\"k\">fn</span>"), "{html}");
    assert!(
        html.contains("<span class=\"s\">&quot;hi&quot;</span>"),
        "{html}"
    );
    assert!(html.contains("<span class=\"c\">// greet</span>"), "{html}");
}

/// A Rust lifetime such as `'a` does not open a character string.
///
/// # Panics
///
/// Panics when anything in `fn f<'a>(x: &'a str)` is marked a string, as it
/// would be if `'a>(x: &'` were read as a quoted character.
#[test]
fn a_lifetime_is_not_a_string() {
    let html = lit("rust", "fn f<'a>(x: &'a str) -> &'a str { x }");
    assert!(!html.contains("class=\"s\""), "{html}");
}

/// A Rust attribute is marked up to its own closing bracket.
///
/// # Panics
///
/// Panics when `#[derive(Debug, [x])]` is not marked whole, stopping at the
/// inner `]` or running past the outer one into `struct S;`, or when an inner
/// attribute `#!x` is not marked as far as its line's end.
#[test]
fn rust_attributes_end_at_their_bracket() {
    let html = lit("rust", "#[derive(Debug, [x])] struct S;\n#!x\ny");
    assert!(
        html.starts_with("<span class=\"m\">#[derive(Debug, [x])]</span> "),
        "{html}"
    );
    assert!(html.contains("<span class=\"m\">#!x</span>\ny"), "{html}");
}

/// A block comment that is never closed runs to the end of the code.
///
/// # Panics
///
/// Panics when `/*/ b` is not one comment to the end, for instance when the
/// `*/` inside `/*/` is taken as its close.
#[test]
fn a_block_comment_without_its_close_runs_to_the_end() {
    let html = lit("go", "a /*/ b");
    assert!(html.ends_with("<span class=\"c\">/*/ b</span>"), "{html}");
}

/// A Python triple-quoted string runs across lines to its closing quotes.
///
/// # Panics
///
/// Panics when `'''a`, an escaped `\'''` and ` b'''` on the next line are not
/// one string, or when the decorator `@deco` after it is not marked.
#[test]
fn python_triple_quotes_run_across_lines() {
    let html = lit("python", "x = '''a\n\\''' b'''\n@deco\n");
    assert!(
        html.contains("<span class=\"s\">'''a\n\\''' b'''</span>"),
        "{html}"
    );
    assert!(html.contains("<span class=\"m\">@deco</span>"), "{html}");
}

/// A quoted string ends at its line's end, except one in backticks.
///
/// # Panics
///
/// Panics when the unclosed `'a` runs on into the next line, or when the
/// backtick string from `` `c `` to `` d` `` is not marked across its line
/// break.
#[test]
fn a_short_string_stops_at_the_line_end_unless_a_backtick_opened_it() {
    let html = lit("js", "'a\nb `c\nd`");
    assert!(html.starts_with("<span class=\"s\">'a</span>\n"), "{html}");
    assert!(html.contains("<span class=\"s\">`c\nd`</span>"), "{html}");
}

/// A C preprocessor line may be indented, but a `#` after code is not one.
///
/// # Panics
///
/// Panics when the indented `#include <a.h>` is not marked a directive (its
/// angle brackets escaped), or when `#` in `x # y` is marked at all.
#[test]
fn a_c_directive_needs_only_white_space_before_it() {
    let html = lit("c", "  #include <a.h>\nx # y");
    assert!(
        html.starts_with("  <span class=\"m\">#include &lt;a.h&gt;</span>"),
        "{html}"
    );
    assert!(html.ends_with("x # y"), "{html}");
}

/// In shell, `#` opens a comment only at the start of a word, and variables are
/// marked.
///
/// # Panics
///
/// Panics when the `#` inside `a#b` starts a comment, when `${HOME}` or `$x` is
/// not marked a variable, or when the closing `# done` is not marked a comment.
#[test]
fn shell_comments_start_a_word_and_variables_are_marked() {
    let html = lit("sh", "echo a#b ${HOME} $x # done");
    assert!(html.contains("a#b"), "{html}");
    assert!(html.contains("<span class=\"t\">${HOME}</span>"), "{html}");
    assert!(html.contains("<span class=\"t\">$x</span>"), "{html}");
    assert!(html.ends_with("<span class=\"c\"># done</span>"), "{html}");
}

/// CSS has its at-rules, properties, colours and measures marked.
///
/// # Panics
///
/// Panics when `@media` is not marked a keyword, `color` a property, or `#fff`
/// and `50%` numbers.
#[test]
fn style_sheets_mark_properties_colours_and_rules() {
    let html = lit("css", "@media x { a { color: #fff; width: 50%; } }");
    assert!(html.contains("<span class=\"k\">@media</span>"), "{html}");
    assert!(html.contains("<span class=\"a\">color</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">#fff</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">50%</span>"), "{html}");
}

/// SQL keywords and functions are known in any case.
///
/// # Panics
///
/// Panics when upper-case `SELECT` is not marked a keyword or mixed-case
/// `Count` a function.
#[test]
fn sql_keywords_ignore_case() {
    let html = lit("sql", "SELECT Count(*) FROM t");
    assert!(html.contains("<span class=\"k\">SELECT</span>"), "{html}");
    assert!(html.contains("<span class=\"t\">Count</span>"), "{html}");
}

/// SQL, which ignores case, lists its words in lower case, the case they are
/// compared in.
///
/// # Panics
///
/// Panics when SQL is not marked as ignoring case, or when any of its listed
/// words has an upper-case letter and so could never match.
#[test]
fn a_language_that_ignores_case_writes_its_tables_in_lower_case() {
    assert!(SQL.has(Feature::NoCase), "SQL ignores case");
    for word in SQL.words() {
        assert_eq!(word, word.to_ascii_lowercase(), "{word}");
    }
}

/// A digit inside a word is part of the word, not a number.
///
/// # Panics
///
/// Panics when the `1` of `x1` is marked a number, when `1.5e3` is not one
/// number, or when `1.x` is marked as more than the number `1`.
#[test]
fn numbers_do_not_start_inside_a_word() {
    let html = lit("go", "x1 1.5e3 1.x");
    assert!(html.starts_with("x1 "), "{html}");
    assert!(html.contains("<span class=\"n\">1.5e3</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">1</span>.x"), "{html}");
}

/// JSON has its keys and its literal values marked.
///
/// # Panics
///
/// Panics when the key `"a"` is not marked a key (its quotes escaped), or
/// `true` a keyword.
#[test]
fn json_keys_and_values() {
    let html = lit("json", "{\"a\": 1, \"b\": true}");
    assert!(
        html.contains("<span class=\"a\">&quot;a&quot;</span>"),
        "{html}"
    );
    assert!(html.contains("<span class=\"k\">true</span>"), "{html}");
}

/// YAML has its keys, quoted values, comments and document markers marked.
///
/// # Panics
///
/// Panics when `a` of `- a: b` is not a key with `# c` a comment after it, when
/// the `#` inside the quoted `'r # s'` starts a comment, when the colon of
/// `http://x` makes another key, when the flow mapping `{a b: c}` is marked, or
/// when the closing `---` is not marked.
#[test]
fn yaml_keys_values_and_comments() {
    let html = lit(
        "yaml",
        "- a: b # c\nq: 'r # s'\nurl: http://x\n{a b: c}\n---",
    );
    assert!(
        html.starts_with("- <span class=\"a\">a</span>: b<span class=\"c\"> # c</span>\n"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"a\">q</span>: <span class=\"s\">'r # s'</span>\n"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"a\">url</span>: http://x\n"),
        "{html}"
    );
    assert!(html.contains("\n{a b: c}\n"), "{html}");
    assert!(html.ends_with("<span class=\"m\">---</span>"), "{html}");
}

/// TOML and INI have their sections, keys, numbers and comments marked.
///
/// # Panics
///
/// Panics when `[a]` is not marked a section, when `b` is not a key with `1.5`
/// a number and `; c` a comment, or when the keyless line `=d` is marked.
#[test]
fn ini_sections_keys_and_numbers() {
    let html = lit("toml", "[a]\nb = 1.5 ; c\n=d\n");
    assert!(html.starts_with("<span class=\"h\">[a]</span>\n"), "{html}");
    assert!(
        html.contains(
            "<span class=\"a\">b </span>= <span class=\"n\">1.5</span><span class=\"c\"> ; c</span>"
        ),
        "{html}"
    );
    assert!(html.ends_with("\n=d\n"), "{html}");
}

/// Each line of a diff is marked by its first character.
///
/// # Panics
///
/// Panics when the output differs at all from the `---` header, the `+`
/// insertion, the `-` deletion and the `@@` hunk line each marked whole, with
/// the context line `e` left plain.
#[test]
fn a_diff_marks_its_lines() {
    let html = lit("diff", "--- a\n+b\n-c\n@@ d\ne");
    assert_eq!(
        html,
        "<span class=\"m\">--- a</span>\n<span class=\"ins\">+b</span>\n<span class=\"del\">-c</span>\n<span class=\"h\">@@ d</span>\ne"
    );
}

/// A console session marks its prompts, the commands after them, and their
/// output.
///
/// # Panics
///
/// Panics when the output differs at all from `$` marked a prompt, `echo` a
/// command and the line `hi` after it output.
#[test]
fn a_console_marks_prompts_and_output() {
    let html = lit("console", "$ echo hi\nhi\n");
    assert_eq!(
        html,
        "<span class=\"m\">$</span> <span class=\"t\">echo</span> hi\n<span class=\"o\">hi</span>\n"
    );
}

/// A Dockerfile has its comments and instructions marked, in any case.
///
/// # Panics
///
/// Panics when the indented `# base` is not one comment, or when lower-case
/// `from` and `run` are not marked keywords.
#[test]
fn a_dockerfile_marks_instructions() {
    let html = lit("dockerfile", "  # base\nfrom x AS y\nrun echo");
    assert!(
        html.starts_with("<span class=\"c\">  # base</span>\n"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"k\">from</span> x AS y"),
        "{html}"
    );
    assert!(html.contains("<span class=\"k\">run</span> "), "{html}");
}

/// HTML has its tags, attribute names and values, and comments marked.
///
/// # Panics
///
/// Panics when `<!-->` is not read as a whole, empty comment, when `<a`, `b`
/// and `"c"` are not marked a tag, an attribute and a string, or when the
/// unclosed `</a` at the end is not marked a tag.
#[test]
fn markup_marks_tags_attributes_and_comments() {
    let html = lit("html", "<!--> x <a b=\"c\" d='e'>f</a");
    assert!(
        html.starts_with("<span class=\"c\">&lt;!--&gt;</span> x "),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"k\">&lt;a</span> <span class=\"a\">b</span>=<span class=\"s\">&quot;c&quot;</span>"),
        "{html}"
    );
    assert!(html.ends_with("f<span class=\"k\">&lt;/a</span>"), "{html}");
}

/// An attribute after a no-break space is still marked as an attribute.
///
/// # Panics
///
/// Panics when `href` after U+00A0 is not marked an attribute name, or the
/// no-break space before it is lost.
#[test]
fn an_attribute_after_a_wide_space_keeps_its_name() {
    let html = lit("html", "<a\u{a0}href=\"x\">");
    assert!(
        html.contains("\u{a0}<span class=\"a\">href</span>="),
        "{html}"
    );
}

/// A language the highlighter does not know gets no highlighting.
///
/// # Panics
///
/// Panics when `nosuchlang` gives highlighted code instead of none.
#[test]
fn unknown_languages_are_plain() {
    assert!(highlight("nosuchlang", "+++").is_none());
}

/// Highlighting adds markup and loses none of the code's text.
///
/// # Panics
///
/// Panics when, for shell, YAML, HTML or a diff, the text of the highlighted
/// code (tags removed, entities decoded, spaces and line breaks aside) is not
/// the code that went in.
#[test]
fn the_text_survives_highlighting() {
    for (lang, code) in [
        ("sh", "echo \"$HOME\" # x <y>\n"),
        ("yaml", "a: b # c\n- d: 1\n"),
        ("html", "<a href=\"x\">y</a>"),
        ("diff", "+a\n-b\n"),
    ] {
        let html = lit(lang, code);
        assert_eq!(
            crate::text::plain(&html.replace('\n', " ")).replace(' ', ""),
            code.replace(['\n', ' '], ""),
            "{lang}"
        );
    }
}
