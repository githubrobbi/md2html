// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! TeX maths as `MathML`, which every current browser draws without a
//! script or a font download. The everyday subset: letters and numbers,
//! operators and relations, `^` and `_`, `\frac`, `\sqrt`, Greek,
//! accents, `\left ... \right`, `\text`, the blackboard and calligraphic
//! alphabets, and the matrix and cases environments. Anything else makes
//! the whole formula fall back to its TeX, shown as code: visible, never
//! guessed.
//!
//! [`parse`] reads the TeX and writes each piece as it is read, with the
//! element builders of [`markup`] and the command tables of [`symbols`].

mod markup;
mod parse;
mod symbols;
#[cfg(test)]
mod tests;

use crate::model::MathMode;
use crate::text::esc;

/// The longest formula read, in bytes: longer ones stay TeX.
const MAX_LEN: usize = 20_000;

/// The `MathML` of a TeX formula, or None when it uses what this does not read.
pub(crate) fn mathml(tex: &str, mode: MathMode) -> Option<String> {
    if tex.len() > MAX_LEN {
        return None;
    }
    let body = parse::formula(tex, mode)?;
    let attr = match mode {
        MathMode::Display => " display=\"block\"",
        MathMode::Inline => "",
    };
    Some(format!(
        "<math{attr}><semantics>{body}<annotation encoding=\"application/x-tex\">{}</annotation></semantics></math>",
        esc(tex)
    ))
}
