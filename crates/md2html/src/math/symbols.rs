// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the commands stand for: the tables that need no parsing, only a
//! name.

/// Functions set upright.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh",
    "log", "ln", "lg", "exp", "det", "dim", "ker", "deg", "arg", "gcd", "Pr", "hom",
];

/// Commands that only adjust size or spacing this does not draw: they
/// stand for nothing.
const IGNORED: &[&str] = &[
    "!",
    "negthinspace",
    "displaystyle",
    "textstyle",
    "limits",
    "nolimits",
    "big",
    "Big",
    "bigg",
    "Bigg",
    "bigl",
    "bigr",
    "Bigl",
    "Bigr",
];

/// The markup of a command that takes no argument, or None when `name` is
/// not one.
pub(super) fn fixed(name: &str) -> Option<String> {
    if let Some(letter) = greek(name) {
        let normal = if letter.chars().next().is_some_and(char::is_uppercase) {
            " mathvariant=\"normal\""
        } else {
            ""
        };
        return Some(format!("<mi{normal}>{letter}</mi>"));
    }
    if let Some(shown) = operator(name) {
        return Some(format!("<mo>{shown}</mo>"));
    }
    if let Some(shown) = identifier(name) {
        return Some(format!("<mi>{shown}</mi>"));
    }
    if FUNCTIONS.contains(&name) {
        return Some(format!("<mi>{name}</mi><mo>\u{2061}</mo>"));
    }
    if let Some(width) = space(name) {
        return Some(format!("<mspace width=\"{width}\"></mspace>"));
    }
    IGNORED.contains(&name).then(String::new)
}

/// A big operator or a limit: what takes its scripts above and below in
/// display style.
pub(super) fn big_operator(name: &str) -> Option<&'static str> {
    Some(match name {
        "sum" => "<mo>∑</mo>",
        "prod" => "<mo>∏</mo>",
        "coprod" => "<mo>∐</mo>",
        "bigcup" => "<mo>⋃</mo>",
        "bigcap" => "<mo>⋂</mo>",
        "bigoplus" => "<mo>⨁</mo>",
        "bigotimes" => "<mo>⨂</mo>",
        "lim" => "<mi>lim</mi>",
        "limsup" => "<mi>lim sup</mi>",
        "liminf" => "<mi>lim inf</mi>",
        "max" => "<mi>max</mi>",
        "min" => "<mi>min</mi>",
        "sup" => "<mi>sup</mi>",
        "inf" => "<mi>inf</mi>",
        _ => return None,
    })
}

/// The mark an accent command sets over its argument.
pub(super) fn accent(name: &str) -> Option<&'static str> {
    Some(match name {
        "hat" | "widehat" => "^",
        "bar" | "overline" => "¯",
        "vec" => "→",
        "dot" => "˙",
        "ddot" => "¨",
        "tilde" | "widetilde" => "~",
        _ => return None,
    })
}

/// The delimiter a `\left` or `\right` names with a character; empty for
/// `.`, which stands for none.
pub(super) const fn fence_char(ch: char) -> Option<&'static str> {
    Some(match ch {
        '.' => "",
        '(' => "(",
        ')' => ")",
        '[' => "[",
        ']' => "]",
        '|' => "|",
        '/' => "/",
        '<' => "⟨",
        '>' => "⟩",
        _ => return None,
    })
}

/// The delimiter a `\left` or `\right` names with a command.
pub(super) fn fence_command(name: &str) -> Option<&'static str> {
    Some(match name {
        "{" | "lbrace" => "{",
        "}" | "rbrace" => "}",
        "|" | "Vert" => "‖",
        "vert" => "|",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        _ => return None,
    })
}

/// What `\begin{name}` sets around and inside its table.
pub(super) struct Environment {
    /// The delimiter before the table, empty for none.
    pub(super) open: &'static str,
    /// The delimiter after the table, empty for none.
    pub(super) close: &'static str,
    /// The attribute that aligns the columns, empty for none.
    pub(super) align: &'static str,
    /// Whether a column spec, `{lcr}`, follows the name.
    pub(super) columns: bool,
}

impl Environment {
    /// A table between `open` and `close`, columns as they come.
    const fn fenced(open: &'static str, close: &'static str) -> Self {
        Self {
            open,
            close,
            align: "",
            columns: false,
        }
    }

    /// A table without delimiters, columns aligned by `align`.
    const fn aligned(align: &'static str) -> Self {
        Self {
            open: "",
            close: "",
            align,
            columns: false,
        }
    }
}

/// The environments this reads.
pub(super) fn environment(name: &str) -> Option<Environment> {
    Some(match name {
        "matrix" | "smallmatrix" | "gathered" | "gather" | "gather*" | "equation" | "equation*" => {
            Environment::fenced("", "")
        }
        "array" => Environment {
            columns: true,
            ..Environment::fenced("", "")
        },
        "aligned" | "align" | "align*" | "split" => {
            Environment::aligned(" columnalign=\"right left\"")
        }
        "pmatrix" => Environment::fenced("(", ")"),
        "bmatrix" => Environment::fenced("[", "]"),
        "Bmatrix" => Environment::fenced("{", "}"),
        "vmatrix" => Environment::fenced("|", "|"),
        "Vmatrix" => Environment::fenced("‖", "‖"),
        "cases" => Environment {
            open: "{",
            ..Environment::aligned(" columnalign=\"left left\"")
        },
        _ => return None,
    })
}

/// The width of a spacing command.
fn space(name: &str) -> Option<&'static str> {
    Some(match name {
        "," | "thinspace" => "0.167em",
        ":" | ">" | "medspace" => "0.222em",
        ";" | "thickspace" => "0.278em",
        " " => "0.25em",
        "quad" => "1em",
        "qquad" => "2em",
        _ => return None,
    })
}

/// A Greek letter.
fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        _ => return None,
    })
}

/// An operator, relation, arrow or delimiter, as the text of its `<mo>`.
fn operator(name: &str) -> Option<&'static str> {
    Some(match name {
        "cdot" => "⋅",
        "times" => "×",
        "div" => "÷",
        "pm" => "±",
        "mp" => "∓",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "∙",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "ne" | "neq" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "propto" => "∝",
        "ll" => "≪",
        "gg" => "≫",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "cup" => "∪",
        "cap" => "∩",
        "setminus" | "smallsetminus" => "∖",
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "to" | "rightarrow" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" | "implies" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" | "iff" => "⇔",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "longrightarrow" => "⟶",
        "longleftarrow" => "⟵",
        "int" => "∫",
        "iint" => "∬",
        "iiint" => "∭",
        "oint" => "∮",
        "partial" => "∂",
        "nabla" => "∇",
        "mid" => "∣",
        "parallel" => "∥",
        "perp" | "bot" => "⊥",
        "angle" => "∠",
        "triangle" => "△",
        "ldots" | "dots" | "dotsc" => "…",
        "cdots" | "dotsb" => "⋯",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "vert" => "|",
        "prime" => "′",
        "colon" => ":",
        "models" => "⊨",
        "vdash" => "⊢",
        "top" => "⊤",
        "wr" => "≀",
        "{" | "lbrace" => "{",
        "}" | "rbrace" => "}",
        "|" | "Vert" => "‖",
        "%" => "%",
        "$" => "$",
        "#" => "#",
        "&" => "&amp;",
        "_" => "_",
        _ => return None,
    })
}

/// A symbol that stands as an identifier.
fn identifier(name: &str) -> Option<&'static str> {
    Some(match name {
        "infty" => "∞",
        "emptyset" | "varnothing" => "∅",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        "wp" => "℘",
        "imath" => "ı",
        "jmath" => "ȷ",
        "degree" => "°",
        _ => return None,
    })
}
