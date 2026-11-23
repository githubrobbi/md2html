// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the highlighter marks, language by language.

use super::highlight;
use super::lang::{Feature, SQL};

/// The code highlighted as `lang`, which must be known.
fn lit(lang: &str, code: &str) -> String {
    highlight(lang, code).expect("a known language")
}

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

#[test]
fn a_lifetime_is_not_a_string() {
    let html = lit("rust", "fn f<'a>(x: &'a str) -> &'a str { x }");
    assert!(!html.contains("class=\"s\""), "{html}");
}

#[test]
fn rust_attributes_end_at_their_bracket() {
    let html = lit("rust", "#[derive(Debug, [x])] struct S;\n#!x\ny");
    assert!(
        html.starts_with("<span class=\"m\">#[derive(Debug, [x])]</span> "),
        "{html}"
    );
    assert!(html.contains("<span class=\"m\">#!x</span>\ny"), "{html}");
}

#[test]
fn a_block_comment_without_its_close_runs_to_the_end() {
    let html = lit("go", "a /*/ b");
    assert!(html.ends_with("<span class=\"c\">/*/ b</span>"), "{html}");
}

#[test]
fn python_triple_quotes_run_across_lines() {
    let html = lit("python", "x = '''a\n\\''' b'''\n@deco\n");
    assert!(
        html.contains("<span class=\"s\">'''a\n\\''' b'''</span>"),
        "{html}"
    );
    assert!(html.contains("<span class=\"m\">@deco</span>"), "{html}");
}

#[test]
fn a_short_string_stops_at_the_line_end_unless_a_backtick_opened_it() {
    let html = lit("js", "'a\nb `c\nd`");
    assert!(html.starts_with("<span class=\"s\">'a</span>\n"), "{html}");
    assert!(html.contains("<span class=\"s\">`c\nd`</span>"), "{html}");
}

#[test]
fn a_c_directive_needs_only_white_space_before_it() {
    let html = lit("c", "  #include <a.h>\nx # y");
    assert!(
        html.starts_with("  <span class=\"m\">#include &lt;a.h&gt;</span>"),
        "{html}"
    );
    assert!(html.ends_with("x # y"), "{html}");
}

#[test]
fn shell_comments_start_a_word_and_variables_are_marked() {
    let html = lit("sh", "echo a#b ${HOME} $x # done");
    assert!(html.contains("a#b"), "{html}");
    assert!(html.contains("<span class=\"t\">${HOME}</span>"), "{html}");
    assert!(html.contains("<span class=\"t\">$x</span>"), "{html}");
    assert!(html.ends_with("<span class=\"c\"># done</span>"), "{html}");
}

#[test]
fn style_sheets_mark_properties_colours_and_rules() {
    let html = lit("css", "@media x { a { color: #fff; width: 50%; } }");
    assert!(html.contains("<span class=\"k\">@media</span>"), "{html}");
    assert!(html.contains("<span class=\"a\">color</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">#fff</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">50%</span>"), "{html}");
}

#[test]
fn sql_keywords_ignore_case() {
    let html = lit("sql", "SELECT Count(*) FROM t");
    assert!(html.contains("<span class=\"k\">SELECT</span>"), "{html}");
    assert!(html.contains("<span class=\"t\">Count</span>"), "{html}");
}

#[test]
fn a_language_that_ignores_case_writes_its_tables_in_lower_case() {
    assert!(SQL.has(Feature::NoCase), "SQL ignores case");
    for word in SQL.words() {
        assert_eq!(word, word.to_ascii_lowercase(), "{word}");
    }
}

#[test]
fn numbers_do_not_start_inside_a_word() {
    let html = lit("go", "x1 1.5e3 1.x");
    assert!(html.starts_with("x1 "), "{html}");
    assert!(html.contains("<span class=\"n\">1.5e3</span>"), "{html}");
    assert!(html.contains("<span class=\"n\">1</span>.x"), "{html}");
}

#[test]
fn json_keys_and_values() {
    let html = lit("json", "{\"a\": 1, \"b\": true}");
    assert!(
        html.contains("<span class=\"a\">&quot;a&quot;</span>"),
        "{html}"
    );
    assert!(html.contains("<span class=\"k\">true</span>"), "{html}");
}

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

#[test]
fn a_diff_marks_its_lines() {
    let html = lit("diff", "--- a\n+b\n-c\n@@ d\ne");
    assert_eq!(
        html,
        "<span class=\"m\">--- a</span>\n<span class=\"ins\">+b</span>\n<span class=\"del\">-c</span>\n<span class=\"h\">@@ d</span>\ne"
    );
}

#[test]
fn a_console_marks_prompts_and_output() {
    let html = lit("console", "$ echo hi\nhi\n");
    assert_eq!(
        html,
        "<span class=\"m\">$</span> <span class=\"t\">echo</span> hi\n<span class=\"o\">hi</span>\n"
    );
}

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

#[test]
fn an_attribute_after_a_wide_space_keeps_its_name() {
    let html = lit("html", "<a\u{a0}href=\"x\">");
    assert!(
        html.contains("\u{a0}<span class=\"a\">href</span>="),
        "{html}"
    );
}

#[test]
fn unknown_languages_are_plain() {
    assert!(highlight("nosuchlang", "+++").is_none());
}

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
