// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the maths reader makes of everyday formulas, and what it refuses.

use super::mathml;
use crate::model::MathMode;

/// The `MathML` of an inline formula, or an empty string when it is refused.
fn inline(tex: &str) -> String {
    mathml(tex, MathMode::Inline).unwrap_or_default()
}

/// The `MathML` of a display formula, or an empty string when it is refused.
fn display(tex: &str) -> String {
    mathml(tex, MathMode::Display).unwrap_or_default()
}

/// Everyday formulas become the `MathML` elements a browser draws them with.
///
/// # Panics
///
/// Panics when `x^2` is not a superscript, `y_i` a subscript or `\frac{a}{b}` a
/// fraction; when a display-style `\sum` does not take its limits above and
/// below, a `pmatrix` is not a table, `\sqrt[3]{x}` is not a root with an
/// index, or `\mathbb{R}` is not the double-struck `ℝ`.
#[test]
fn everyday_formulas() {
    let html = inline("x^2 + y_i = \\frac{a}{b}");
    assert!(html.contains("<msup><mi>x</mi><mn>2</mn></msup>"), "{html}");
    assert!(html.contains("<msub><mi>y</mi><mi>i</mi></msub>"), "{html}");
    assert!(
        html.contains("<mfrac><mi>a</mi><mi>b</mi></mfrac>"),
        "{html}"
    );
    assert!(
        display("\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}").contains("munderover"),
        "a sum in display style takes its limits above and below"
    );
    assert!(
        display("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}").contains("<mtable>"),
        "a matrix is a table"
    );
    assert!(
        inline("\\left( \\sqrt[3]{x} \\right)").contains("<mroot>"),
        "a root with an index"
    );
    assert!(inline("\\mathbb{R}^n").contains('ℝ'), "the double-struck R");
}

/// Row breaks, cells, fences, primes and letter styles come out as written.
///
/// # Panics
///
/// Panics when a trailing `\\` adds an empty row, when `cases` does not open
/// with a stretching brace and align its columns left (the `&` of its TeX
/// escaped in the annotation), when `\left.` does not leave the left fence
/// empty beside a stretching `⟩`, when `f''` does not give a double prime, or
/// when `\mathcal{L}` and `\mathbf{v2}` do not give the script `ℒ` and the bold
/// `𝐯𝟐`.
#[test]
fn rows_cells_and_fences() {
    assert_eq!(
        inline("a \\\\ b \\\\"),
        "<math><semantics><mtable displaystyle=\"true\"><mtr><mtd><mi>a</mi></mtd></mtr>\
         <mtr><mtd><mi>b</mi></mtd></mtr></mtable>\
         <annotation encoding=\"application/x-tex\">a \\\\ b \\\\</annotation></semantics></math>",
        "a trailing row break adds no empty row"
    );
    assert_eq!(
        inline("\\begin{cases} 1 & x \\end{cases}"),
        "<math><semantics><mrow><mo stretchy=\"true\">{</mo>\
         <mtable columnalign=\"left left\"><mtr><mtd><mn>1</mn></mtd><mtd><mi>x</mi></mtd></mtr></mtable>\
         </mrow><annotation encoding=\"application/x-tex\">\\begin{cases} 1 &amp; x \\end{cases}</annotation>\
         </semantics></math>",
        "cases open with a brace and align left"
    );
    assert!(
        inline("\\left. x \\right\\rangle").contains("<mi>x</mi><mo stretchy=\"true\">⟩</mo>"),
        "an empty left fence and a named right one"
    );
    assert!(inline("f''(x) \\& \\{").contains("<mo>′′</mo>"), "primes");
    assert!(
        inline("\\mathcal{L} \\mathbf{v2}").contains("<mi>ℒ</mi><mi>𝐯𝟐</mi>"),
        "the script and bold alphabets"
    );
}

/// A formula the reader cannot read is refused whole, never guessed.
///
/// # Panics
///
/// Panics when any of these gives `MathML` instead of nothing: an unknown
/// command, a stray `}`, `\frac` with one argument, `&` outside a table, an
/// environment closed by another name, `\mathcal` of a digit, 80 nested braces,
/// or `\right` without `\left`.
#[test]
fn what_it_cannot_read_falls_back() {
    assert!(
        mathml("\\unknowncommand{x}", MathMode::Inline).is_none(),
        "an unknown command"
    );
    assert!(mathml("a } b", MathMode::Inline).is_none(), "a stray brace");
    assert!(
        mathml("\\frac{a}", MathMode::Inline).is_none(),
        "a missing argument"
    );
    assert!(
        mathml("a & b", MathMode::Inline).is_none(),
        "cells outside a table"
    );
    assert!(
        mathml("\\begin{matrix} a \\end{pmatrix}", MathMode::Inline).is_none(),
        "an environment closed by another name"
    );
    assert!(
        mathml("\\mathcal{3}", MathMode::Inline).is_none(),
        "script has no digits"
    );
    assert!(
        mathml(&"{".repeat(80), MathMode::Inline).is_none(),
        "nesting past the limit"
    );
    assert!(
        mathml("x \\right)", MathMode::Inline).is_none(),
        "a lone right fence"
    );
}
