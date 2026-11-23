// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::indexing_slicing,
    clippy::min_ident_chars,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

//! TeX maths as `MathML`, which every current browser draws without a
//! script or a font download. The everyday subset: letters and numbers,
//! operators and relations, `^` and `_`, `\frac`, `\sqrt`, Greek,
//! accents, `\left ... \right`, `\text`, the blackboard and calligraphic
//! alphabets, and the matrix and cases environments. Anything else makes
//! the whole formula fall back to its TeX, shown as code: visible, never
//! guessed.
//!
//! Exception: the `TeX` subset and its `MathML` writer, a single
//! recursive-descent parser (`scripts/ci/file_size_exceptions.txt`).

use crate::text::esc;

/// The `MathML` of a TeX formula, or None when it uses what this does not read.
pub(crate) fn mathml(tex: &str, display: bool) -> Option<String> {
    if tex.len() > 20_000 {
        return None;
    }
    let mut p = Parser {
        s: tex.chars().collect(),
        i: 0,
        depth: 0,
        display,
    };
    let rows = p.rows(Stop::End)?;
    let body = if rows.len() > 1 {
        let mut table = String::from("<mtable displaystyle=\"true\">");
        for row in rows {
            table.push_str("<mtr>");
            for cell in row {
                table.push_str("<mtd>");
                table.push_str(&mrow(cell));
                table.push_str("</mtd>");
            }
            table.push_str("</mtr>");
        }
        table.push_str("</mtable>");
        table
    } else {
        let cells = rows.into_iter().next().unwrap_or_default();
        if cells.len() > 1 {
            return None;
        }
        mrow(cells.into_iter().next().unwrap_or_default())
    };
    let attr = if display { " display=\"block\"" } else { "" };
    Some(format!(
        "<math{attr}><semantics>{body}<annotation encoding=\"application/x-tex\">{}</annotation></semantics></math>",
        esc(tex)
    ))
}

/// Elements as one: a single one alone, several in an `<mrow>`.
fn mrow(items: Vec<String>) -> String {
    match items.len() {
        0 => String::from("<mrow></mrow>"),
        1 => items.into_iter().next().unwrap_or_default(),
        _ => format!("<mrow>{}</mrow>", items.concat()),
    }
}

/// What ends a run of elements.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// The end of the formula.
    End,
    /// `}`.
    Brace,
    /// `]` (an optional argument).
    Bracket,
    /// `\right`.
    Right,
    /// `\end{...}`.
    Env,
}

/// The parser over the formula's characters.
struct Parser {
    /// The characters.
    s: Vec<char>,
    /// Where it is.
    i: usize,
    /// Nesting, bounded.
    depth: usize,
    /// Display style: big operators take their limits above and below.
    display: bool,
}

impl Parser {
    /// The next character, not consumed.
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    /// Passes over white space.
    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    /// A command's name after its backslash.
    fn name(&mut self) -> String {
        let start = self.i;
        if self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                self.i += 1;
            }
        } else if self.peek().is_some() {
            self.i += 1;
        }
        self.s[start..self.i].iter().collect()
    }

    /// Rows of cells (split at `\\` and `&`) up to `stop`.
    fn rows(&mut self, stop: Stop) -> Option<Vec<Vec<Vec<String>>>> {
        self.depth += 1;
        if self.depth > 60 {
            return None;
        }
        let mut rows: Vec<Vec<Vec<String>>> = vec![vec![Vec::new()]];
        loop {
            self.skip_space();
            let Some(c) = self.peek() else {
                if stop == Stop::End {
                    break;
                }
                return None;
            };
            match c {
                '}' if stop == Stop::Brace => {
                    self.i += 1;
                    break;
                }
                ']' if stop == Stop::Bracket => {
                    self.i += 1;
                    break;
                }
                '}' => return None,
                '&' => {
                    self.i += 1;
                    rows.last_mut()?.push(Vec::new());
                    continue;
                }
                '\\' if self.s.get(self.i + 1) == Some(&'\\') => {
                    self.i += 2;
                    // An optional spacing argument: \\[2pt].
                    if self.peek() == Some('[') {
                        while self.peek().is_some_and(|c| c != ']') {
                            self.i += 1;
                        }
                        self.i += 1;
                    }
                    rows.push(vec![Vec::new()]);
                    continue;
                }
                '\\' => {
                    let save = self.i;
                    self.i += 1;
                    let name = self.name();
                    if name == "right" && stop == Stop::Right {
                        break;
                    }
                    if name == "end" && stop == Stop::Env {
                        self.i = save;
                        break;
                    }
                    if name == "right" || name == "end" {
                        return None;
                    }
                    self.i = save;
                }
                _ => {}
            }
            let atom = self.scripted()?;
            rows.last_mut()?.last_mut()?.push(atom);
        }
        self.depth -= 1;
        // A trailing `\\` leaves an empty row behind.
        if rows.len() > 1 && rows.last().is_some_and(|r| r.len() == 1 && r[0].is_empty()) {
            rows.pop();
        }
        Some(rows)
    }

    /// The elements up to `stop` as one row: no `&` or `\\` allowed.
    fn group(&mut self, stop: Stop) -> Option<String> {
        let rows = self.rows(stop)?;
        if rows.len() != 1 || rows[0].len() != 1 {
            return None;
        }
        Some(mrow(rows.into_iter().next()?.into_iter().next()?))
    }

    /// One argument: a `{group}` or a single token.
    fn arg(&mut self) -> Option<String> {
        self.skip_space();
        if self.peek() == Some('{') {
            self.i += 1;
            return self.group(Stop::Brace);
        }
        self.atom()
    }

    /// An argument's raw text: `{text}`.
    fn raw_arg(&mut self) -> Option<String> {
        self.skip_space();
        if self.peek() != Some('{') {
            return self.peek().map(|c| {
                self.i += 1;
                c.to_string()
            });
        }
        self.i += 1;
        let start = self.i;
        let mut depth = 1_i32;
        while let Some(c) = self.peek() {
            match c {
                '{' => depth += 1_i32,
                '}' => {
                    depth -= 1_i32;
                    if depth == 0_i32 {
                        let text: String = self.s[start..self.i].iter().collect();
                        self.i += 1;
                        return Some(text);
                    }
                }
                '\\' => self.i += 1,
                _ => {}
            }
            self.i += 1;
        }
        None
    }

    /// An atom with its `^` and `_`.
    fn scripted(&mut self) -> Option<String> {
        let (base, big) = self.base()?;
        let mut sub = None;
        let mut sup = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some('^') if sup.is_none() => {
                    self.i += 1;
                    sup = Some(self.arg()?);
                }
                Some('_') if sub.is_none() => {
                    self.i += 1;
                    sub = Some(self.arg()?);
                }
                Some('\'') if sup.is_none() => {
                    let mut primes = String::new();
                    while self.peek() == Some('\'') {
                        self.i += 1;
                        primes.push('′');
                    }
                    sup = Some(format!("<mo>{primes}</mo>"));
                }
                _ => break,
            }
        }
        let (under, over, both) = if big && self.display {
            ("munder", "mover", "munderover")
        } else {
            ("msub", "msup", "msubsup")
        };
        Some(match (sub, sup) {
            (None, None) => base,
            (Some(b), None) => format!("<{under}>{base}{b}</{under}>"),
            (None, Some(p)) => format!("<{over}>{base}{p}</{over}>"),
            (Some(b), Some(p)) => format!("<{both}>{base}{b}{p}</{both}>"),
        })
    }

    /// The base of an atom, and whether it takes its limits above and below.
    fn base(&mut self) -> Option<(String, bool)> {
        self.skip_space();
        if self.peek() == Some('\\') {
            let save = self.i;
            self.i += 1;
            let name = self.name();
            if matches!(
                name.as_str(),
                "sum"
                    | "prod"
                    | "coprod"
                    | "bigcup"
                    | "bigcap"
                    | "bigoplus"
                    | "bigotimes"
                    | "lim"
                    | "limsup"
                    | "liminf"
                    | "max"
                    | "min"
                    | "sup"
                    | "inf"
            ) {
                let shown = match name.as_str() {
                    "sum" => "<mo>∑</mo>".to_owned(),
                    "prod" => "<mo>∏</mo>".to_owned(),
                    "coprod" => "<mo>∐</mo>".to_owned(),
                    "bigcup" => "<mo>⋃</mo>".to_owned(),
                    "bigcap" => "<mo>⋂</mo>".to_owned(),
                    "bigoplus" => "<mo>⨁</mo>".to_owned(),
                    "bigotimes" => "<mo>⨂</mo>".to_owned(),
                    "limsup" => "<mi>lim sup</mi>".to_owned(),
                    "liminf" => "<mi>lim inf</mi>".to_owned(),
                    other => format!("<mi>{other}</mi>"),
                };
                return Some((shown, true));
            }
            self.i = save;
        }
        if self.peek() == Some('^') || self.peek() == Some('_') {
            return Some((String::from("<mrow></mrow>"), false));
        }
        Some((self.atom()?, false))
    }

    /// One token or command.
    fn atom(&mut self) -> Option<String> {
        self.skip_space();
        let c = self.peek()?;
        self.i += 1;
        match c {
            '{' => self.group(Stop::Brace),
            '0'..='9' | '.' => {
                let start = self.i - 1;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.i += 1;
                }
                let n: String = self.s[start..self.i].iter().collect();
                if n == "." {
                    return Some(String::from("<mo>.</mo>"));
                }
                Some(format!("<mn>{n}</mn>"))
            }
            '+' | '=' | '<' | '>' | '(' | ')' | '[' | ']' | '|' | '/' | ',' | ';' | ':' | '!'
            | '*' | '?' => {
                let shown = match c {
                    '<' => "&lt;".to_owned(),
                    '>' => "&gt;".to_owned(),
                    '*' => "∗".to_owned(),
                    other => other.to_string(),
                };
                let stretchy = if matches!(c, '(' | ')' | '[' | ']' | '|') {
                    " stretchy=\"false\""
                } else {
                    ""
                };
                Some(format!("<mo{stretchy}>{shown}</mo>"))
            }
            '-' => Some(String::from("<mo>−</mo>")),
            '~' => Some(String::from("<mspace width=\"0.33em\"></mspace>")),
            '}' | '&' | '^' | '_' | '#' | '%' | '$' => None,
            '\\' => {
                let name = self.name();
                self.command(&name)
            }
            ch if ch.is_alphabetic() => Some(format!("<mi>{}</mi>", esc(&ch.to_string()))),
            ch => Some(format!("<mo>{}</mo>", esc(&ch.to_string()))),
        }
    }

    /// A command after its backslash.
    fn command(&mut self, name: &str) -> Option<String> {
        if let Some(letter) = greek(name) {
            let normal = if letter.chars().next().is_some_and(char::is_uppercase) {
                " mathvariant=\"normal\""
            } else {
                ""
            };
            return Some(format!("<mi{normal}>{letter}</mi>"));
        }
        if let Some(op) = symbol(name) {
            return Some(format!("<mo>{op}</mo>"));
        }
        if let Some(ident) = ident_symbol(name) {
            return Some(format!("<mi>{ident}</mi>"));
        }
        if FUNCTIONS.contains(&name) {
            return Some(format!("<mi>{name}</mi><mo>\u{2061}</mo>"));
        }
        Some(match name {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let a = self.arg()?;
                let b = self.arg()?;
                format!("<mfrac>{a}{b}</mfrac>")
            }
            "binom" | "dbinom" | "tbinom" => {
                let a = self.arg()?;
                let b = self.arg()?;
                format!(
                    "<mrow><mo>(</mo><mfrac linethickness=\"0\">{a}{b}</mfrac><mo>)</mo></mrow>"
                )
            }
            "sqrt" => {
                self.skip_space();
                if self.peek() == Some('[') {
                    self.i += 1;
                    let index = self.group(Stop::Bracket)?;
                    let body = self.arg()?;
                    format!("<mroot>{body}{index}</mroot>")
                } else {
                    format!("<msqrt>{}</msqrt>", self.arg()?)
                }
            }
            "text" | "textrm" | "textnormal" | "mbox" | "textit" | "textbf" | "texttt" => {
                let text = self.raw_arg()?;
                format!("<mtext>{}</mtext>", esc(&text.replace(['{', '}'], "")))
            }
            "mathrm" | "operatorname" | "rm" => {
                let text = self.raw_arg()?;
                format!(
                    "<mi mathvariant=\"normal\">{}</mi>",
                    esc(&text.replace(['{', '}', '\\', ' '], ""))
                )
            }
            "mathbf" | "bf" | "boldsymbol" | "bm" => {
                let text = self.raw_arg()?;
                styled(&text, Style::Bold)?
            }
            "mathbb" => styled(&self.raw_arg()?, Style::Double)?,
            "mathcal" | "mathscr" => styled(&self.raw_arg()?, Style::Script)?,
            "mathit" => {
                let text = self.raw_arg()?;
                format!("<mi>{}</mi>", esc(&text))
            }
            "hat" | "widehat" => accent(&self.arg()?, "^"),
            "bar" | "overline" => accent(&self.arg()?, "¯"),
            "vec" => accent(&self.arg()?, "→"),
            "dot" => accent(&self.arg()?, "˙"),
            "ddot" => accent(&self.arg()?, "¨"),
            "tilde" | "widetilde" => accent(&self.arg()?, "~"),
            "underline" => format!(
                "<munder accentunder=\"true\">{}<mo>_</mo></munder>",
                self.arg()?
            ),
            "left" => {
                let open = self.fence()?;
                let body = self.group(Stop::Right)?;
                let close = self.fence()?;
                format!("<mrow>{open}{body}{close}</mrow>")
            }
            "begin" => self.environment()?,
            "," | "thinspace" => String::from("<mspace width=\"0.167em\"></mspace>"),
            ":" | ">" | "medspace" => String::from("<mspace width=\"0.222em\"></mspace>"),
            ";" | "thickspace" => String::from("<mspace width=\"0.278em\"></mspace>"),
            " " => String::from("<mspace width=\"0.25em\"></mspace>"),
            "quad" => String::from("<mspace width=\"1em\"></mspace>"),
            "qquad" => String::from("<mspace width=\"2em\"></mspace>"),
            "{" | "lbrace" => String::from("<mo>{</mo>"),
            "}" | "rbrace" => String::from("<mo>}</mo>"),
            "|" => String::from("<mo>‖</mo>"),
            "%" | "$" | "#" | "&" | "_" => format!("<mo>{}</mo>", esc(name)),
            "!" | "negthinspace" | "displaystyle" | "textstyle" | "limits" | "nolimits" | "big"
            | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" => String::new(),
            _ => return None,
        })
    }

    /// The delimiter after `\left` or `\right`.
    fn fence(&mut self) -> Option<String> {
        self.skip_space();
        let c = self.peek()?;
        self.i += 1;
        let shown = match c {
            '.' => return Some(String::new()),
            '(' | ')' | '[' | ']' | '|' | '/' => c.to_string(),
            '<' => "⟨".to_owned(),
            '>' => "⟩".to_owned(),
            '\\' => match self.name().as_str() {
                "{" | "lbrace" => "{".to_owned(),
                "}" | "rbrace" => "}".to_owned(),
                "|" | "Vert" => "‖".to_owned(),
                "vert" => "|".to_owned(),
                "langle" => "⟨".to_owned(),
                "rangle" => "⟩".to_owned(),
                "lfloor" => "⌊".to_owned(),
                "rfloor" => "⌋".to_owned(),
                "lceil" => "⌈".to_owned(),
                "rceil" => "⌉".to_owned(),
                _ => return None,
            },
            _ => return None,
        };
        Some(format!("<mo stretchy=\"true\">{}</mo>", esc(&shown)))
    }

    /// `\begin{name} ... \end{name}`.
    fn environment(&mut self) -> Option<String> {
        let name = self.raw_arg()?;
        let (open, close) = match name.as_str() {
            "matrix" | "smallmatrix" | "aligned" | "align" | "align*" | "gathered" | "gather"
            | "gather*" | "split" | "array" | "equation" | "equation*" => ("", ""),
            "pmatrix" => ("(", ")"),
            "bmatrix" => ("[", "]"),
            "Bmatrix" => ("{", "}"),
            "vmatrix" => ("|", "|"),
            "Vmatrix" => ("‖", "‖"),
            "cases" => ("{", ""),
            _ => return None,
        };
        if name == "array" {
            // The column spec: {lcr}.
            self.raw_arg()?;
        }
        let rows = self.rows(Stop::Env)?;
        // `\end{name}`
        self.skip_space();
        if self.peek() != Some('\\') {
            return None;
        }
        self.i += 1;
        if self.name() != "end" {
            return None;
        }
        if self.raw_arg()? != name {
            return None;
        }
        let align = match name.as_str() {
            "aligned" | "align" | "align*" | "split" => " columnalign=\"right left\"",
            "cases" => " columnalign=\"left left\"",
            _ => "",
        };
        let mut table = format!("<mtable{align}>");
        for row in rows {
            table.push_str("<mtr>");
            for cell in row {
                table.push_str("<mtd>");
                table.push_str(&mrow(cell));
                table.push_str("</mtd>");
            }
            table.push_str("</mtr>");
        }
        table.push_str("</mtable>");
        let fence = |c: &str| {
            if c.is_empty() {
                String::new()
            } else {
                format!("<mo stretchy=\"true\">{c}</mo>")
            }
        };
        Some(format!(
            "<mrow>{}{table}{}</mrow>",
            fence(open),
            fence(close)
        ))
    }
}

/// An accent over an expression.
fn accent(body: &str, mark: &str) -> String {
    format!("<mover accent=\"true\">{body}<mo>{mark}</mo></mover>")
}

/// The alphabets of `\mathbf`, `\mathbb`, `\mathcal`.
#[derive(Clone, Copy)]
enum Style {
    /// Bold.
    Bold,
    /// Double-struck.
    Double,
    /// Script.
    Script,
}

/// Letters in a mathematical alphabet, as the Unicode characters for it.
fn styled(text: &str, style: Style) -> Option<String> {
    let mut out = String::new();
    for ch in text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '{' && *c != '}')
    {
        let mapped = match (style, ch) {
            (Style::Double, 'C') => 'ℂ',
            (Style::Double, 'H') => 'ℍ',
            (Style::Double, 'N') => 'ℕ',
            (Style::Double, 'P') => 'ℙ',
            (Style::Double, 'Q') => 'ℚ',
            (Style::Double, 'R') => 'ℝ',
            (Style::Double, 'Z') => 'ℤ',
            (Style::Script, 'B') => 'ℬ',
            (Style::Script, 'E') => 'ℰ',
            (Style::Script, 'F') => 'ℱ',
            (Style::Script, 'H') => 'ℋ',
            (Style::Script, 'I') => 'ℐ',
            (Style::Script, 'L') => 'ℒ',
            (Style::Script, 'M') => 'ℳ',
            (Style::Script, 'R') => 'ℛ',
            (style, c) if c.is_ascii_uppercase() || c.is_ascii_lowercase() => {
                let base = match style {
                    Style::Bold => 0x1D400,
                    Style::Script => 0x1D49C,
                    Style::Double => 0x1D538,
                };
                let offset = if c.is_ascii_uppercase() {
                    u32::from(c) - u32::from('A')
                } else {
                    26 + u32::from(c) - u32::from('a')
                };
                char::from_u32(base + offset)?
            }
            (Style::Bold, c) if c.is_ascii_digit() => {
                char::from_u32(0x1D7CE + u32::from(c) - u32::from('0'))?
            }
            (Style::Double, c) if c.is_ascii_digit() => {
                char::from_u32(0x1D7D8 + u32::from(c) - u32::from('0'))?
            }
            _ => return None,
        };
        out.push(mapped);
    }
    Some(format!("<mi>{out}</mi>"))
}

/// Functions set upright.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh",
    "log", "ln", "lg", "exp", "det", "dim", "ker", "deg", "arg", "gcd", "Pr", "hom",
];

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

/// An operator, relation or arrow.
fn symbol(name: &str) -> Option<&'static str> {
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
        "Vert" => "‖",
        "prime" => "′",
        "colon" => ":",
        "models" => "⊨",
        "vdash" => "⊢",
        "top" => "⊤",
        "wr" => "≀",
        _ => return None,
    })
}

/// A symbol that stands as an identifier.
fn ident_symbol(name: &str) -> Option<&'static str> {
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

#[cfg(test)]
mod tests {
    use super::mathml;

    #[test]
    fn everyday_formulas() {
        let html = mathml("x^2 + y_i = \\frac{a}{b}", false).unwrap_or_default();
        assert!(html.contains("<msup><mi>x</mi><mn>2</mn></msup>"), "{html}");
        assert!(html.contains("<msub><mi>y</mi><mi>i</mi></msub>"), "{html}");
        assert!(
            html.contains("<mfrac><mi>a</mi><mi>b</mi></mfrac>"),
            "{html}"
        );
        assert!(
            mathml("\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}", true)
                .is_some_and(|h| h.contains("munderover"))
        );
        assert!(
            mathml("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}", true)
                .is_some_and(|h| h.contains("<mtable>"))
        );
        assert!(
            mathml("\\left( \\sqrt[3]{x} \\right)", false).is_some_and(|h| h.contains("<mroot>"))
        );
        assert!(mathml("\\mathbb{R}^n", false).is_some_and(|h| h.contains('ℝ')));
    }

    #[test]
    fn what_it_cannot_read_falls_back() {
        assert!(mathml("\\unknowncommand{x}", false).is_none());
        assert!(mathml("a } b", false).is_none());
        assert!(mathml("\\frac{a}", false).is_none());
    }
}
