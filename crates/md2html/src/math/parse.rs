// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The recursive-descent reader of the TeX subset. Each piece is written
//! as `MathML` as soon as it is read; anything outside the subset makes
//! the whole formula None.

use super::markup::{self, Cell, Row, Style};
use super::symbols;
use crate::model::MathMode;
use crate::scan::Scan;
use crate::text::esc;

/// How deep groups may nest before the formula is given up.
const MAX_DEPTH: usize = 60;

/// The `MathML` of a whole formula: one row, or a table when `\\` splits
/// it into several.
pub(super) fn formula(tex: &str, mode: MathMode) -> Option<String> {
    let mut parser = Parser {
        scan: Scan::new(tex),
        depth: 0,
        mode,
    };
    match <[Row; 1]>::try_from(parser.rows(Stop::End)?) {
        Ok([row]) => {
            let [cell] = <[Cell; 1]>::try_from(row).ok()?;
            Some(markup::mrow(&cell))
        }
        Err(rows) => Some(markup::table(&rows, " displaystyle=\"true\"")),
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

/// What comes next in a run of elements.
enum Next {
    /// The run is over.
    Stop,
    /// `&`: a new cell.
    Cell,
    /// `\\`: a new row.
    Row,
    /// An element.
    Element,
}

/// Rows of cells as they are read. The open row and its open cell are
/// always there, so an element always has a place to go.
#[derive(Default)]
struct Grid {
    /// The finished rows.
    rows: Vec<Row>,
    /// The finished cells of the open row.
    cells: Row,
    /// The open cell.
    cell: Cell,
}

impl Grid {
    /// Adds an element to the open cell.
    fn push(&mut self, element: String) {
        self.cell.push(element);
    }

    /// Closes the open cell and opens the next one.
    fn next_cell(&mut self) {
        self.cells.push(core::mem::take(&mut self.cell));
    }

    /// Closes the open row and opens the next one.
    fn next_row(&mut self) {
        self.next_cell();
        self.rows.push(core::mem::take(&mut self.cells));
    }

    /// All the rows. A trailing `\\` leaves an empty row open, which is
    /// dropped.
    fn finish(mut self) -> Vec<Row> {
        let trailing = !self.rows.is_empty() && self.cells.is_empty() && self.cell.is_empty();
        if !trailing {
            self.next_row();
        }
        self.rows
    }
}

/// The parser over the formula's text.
struct Parser<'tex> {
    /// The formula, read from the front.
    scan: Scan<'tex>,
    /// Nesting, bounded.
    depth: usize,
    /// In the line, or set apart; set apart, big operators take their
    /// limits above and below.
    mode: MathMode,
}

/// A command's name, read after its backslash: a run of ASCII letters, or
/// any single character. Empty at the end of the text.
fn command_name<'tex>(scan: &mut Scan<'tex>) -> &'tex str {
    let letters = scan.take_while(|ch| ch.is_ascii_alphabetic());
    if letters.is_empty() {
        scan.take_chars(1)
    } else {
        letters
    }
}

impl<'tex> Parser<'tex> {
    /// Passes over white space.
    fn skip_space(&mut self) {
        self.scan.take_while(char::is_whitespace);
    }

    /// The name of the command that is next, not read, and the reader just
    /// past it. None when no command is next.
    fn command_ahead(&self) -> Option<(&'tex str, Scan<'tex>)> {
        let mut ahead = self.scan.clone();
        if !ahead.eat('\\') {
            return None;
        }
        let name = command_name(&mut ahead);
        Some((name, ahead))
    }

    /// Rows of cells (split at `\\` and `&`) up to `stop`.
    fn rows(&mut self, stop: Stop) -> Option<Vec<Row>> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut grid = Grid::default();
        loop {
            self.skip_space();
            match self.next(stop)? {
                Next::Stop => break,
                Next::Cell => grid.next_cell(),
                Next::Row => grid.next_row(),
                Next::Element => grid.push(self.scripted()?),
            }
        }
        self.depth -= 1;
        Some(grid.finish())
    }

    /// Reads what ends the run, a cell or a row, if that is next; None
    /// when what is next has no place in a run that ends at `stop`.
    fn next(&mut self, stop: Stop) -> Option<Next> {
        let Some(ch) = self.scan.peek() else {
            return (stop == Stop::End).then_some(Next::Stop);
        };
        match ch {
            '}' => {
                self.scan.bump();
                (stop == Stop::Brace).then_some(Next::Stop)
            }
            ']' if stop == Stop::Bracket => {
                self.scan.bump();
                Some(Next::Stop)
            }
            '&' => {
                self.scan.bump();
                Some(Next::Cell)
            }
            '\\' => self.next_command(stop),
            _ => Some(Next::Element),
        }
    }

    /// [`Parser::next`] at a backslash: `\\`, `\right` and `\end` end
    /// something, any other command is an element.
    fn next_command(&mut self, stop: Stop) -> Option<Next> {
        if self.scan.eat_str("\\\\") {
            // An optional spacing argument: \\[2pt].
            if self.scan.peek() == Some('[') {
                self.scan.take_while(|ch| ch != ']');
                self.scan.bump();
            }
            return Some(Next::Row);
        }
        match self.command_ahead() {
            Some(("right", after)) if stop == Stop::Right => {
                self.scan = after;
                Some(Next::Stop)
            }
            // `\end{name}` stays unread: the environment checks the name.
            Some(("end", _)) if stop == Stop::Env => Some(Next::Stop),
            Some(("right" | "end", _)) => None,
            _ => Some(Next::Element),
        }
    }

    /// The elements up to `stop` as one row: no `&` or `\\` allowed.
    fn group(&mut self, stop: Stop) -> Option<String> {
        let [row] = <[Row; 1]>::try_from(self.rows(stop)?).ok()?;
        let [cell] = <[Cell; 1]>::try_from(row).ok()?;
        Some(markup::mrow(&cell))
    }

    /// One argument: a `{group}` or a single token.
    fn arg(&mut self) -> Option<String> {
        self.skip_space();
        if self.scan.eat('{') {
            return self.group(Stop::Brace);
        }
        self.atom()
    }

    /// An argument's raw text: what is inside `{...}`, nested braces
    /// included, or a single character.
    fn raw_arg(&mut self) -> Option<&'tex str> {
        self.skip_space();
        let start = self.scan.mark();
        if !self.scan.eat('{') {
            self.scan.bump()?;
            return Some(self.scan.since(start));
        }
        let inside = self.scan.mark();
        let mut depth = 1_u32;
        while let Some(ch) = self.scan.peek() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let text = self.scan.since(inside);
                        self.scan.bump();
                        return Some(text);
                    }
                }
                '\\' => {
                    self.scan.bump();
                }
                _ => {}
            }
            self.scan.bump();
        }
        None
    }

    /// An atom with its `^` and `_`.
    fn scripted(&mut self) -> Option<String> {
        self.skip_space();
        let (base, limits) = match self.big_operator() {
            Some(operator) => (operator.to_owned(), self.mode == MathMode::Display),
            None => (self.base()?, false),
        };
        let mut sub = None;
        let mut sup = None;
        loop {
            self.skip_space();
            match self.scan.peek() {
                Some('^') if sup.is_none() => {
                    self.scan.bump();
                    sup = Some(self.arg()?);
                }
                Some('_') if sub.is_none() => {
                    self.scan.bump();
                    sub = Some(self.arg()?);
                }
                Some('\'') if sup.is_none() => {
                    let primes = self.scan.take_while(|ch| ch == '\'');
                    sup = Some(format!("<mo>{}</mo>", "′".repeat(primes.len())));
                }
                _ => break,
            }
        }
        Some(markup::scripts(base, sub, sup, limits))
    }

    /// Reads a big operator or a limit, if one is next.
    fn big_operator(&mut self) -> Option<&'static str> {
        let (name, after) = self.command_ahead()?;
        let shown = symbols::big_operator(name)?;
        self.scan = after;
        Some(shown)
    }

    /// The base of a script: an atom, or nothing before a bare `^` or `_`.
    fn base(&mut self) -> Option<String> {
        if matches!(self.scan.peek(), Some('^' | '_')) {
            return Some(String::from("<mrow></mrow>"));
        }
        self.atom()
    }

    /// One token or command.
    fn atom(&mut self) -> Option<String> {
        self.skip_space();
        let digits = self.scan.take_while(|ch| ch.is_ascii_digit() || ch == '.');
        if !digits.is_empty() {
            return Some(markup::number(digits));
        }
        match self.scan.bump()? {
            '{' => self.group(Stop::Brace),
            '\\' => {
                let name = command_name(&mut self.scan);
                self.command(name)
            }
            ch => markup::character(ch),
        }
    }

    /// A command after its backslash.
    fn command(&mut self, name: &str) -> Option<String> {
        if let Some(fixed) = symbols::fixed(name) {
            return Some(fixed);
        }
        if let Some(mark) = symbols::accent(name) {
            return Some(markup::accent(&self.arg()?, mark));
        }
        match name {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let numerator = self.arg()?;
                let denominator = self.arg()?;
                Some(format!("<mfrac>{numerator}{denominator}</mfrac>"))
            }
            "binom" | "dbinom" | "tbinom" => {
                let top = self.arg()?;
                let bottom = self.arg()?;
                Some(format!(
                    "<mrow><mo>(</mo><mfrac linethickness=\"0\">{top}{bottom}</mfrac><mo>)</mo></mrow>"
                ))
            }
            "sqrt" => self.root(),
            "text" | "textrm" | "textnormal" | "mbox" | "textit" | "textbf" | "texttt" => {
                let text = self.raw_arg()?.replace(['{', '}'], "");
                Some(format!("<mtext>{}</mtext>", esc(&text)))
            }
            "mathrm" | "operatorname" | "rm" => {
                let text = self.raw_arg()?.replace(['{', '}', '\\', ' '], "");
                Some(format!("<mi mathvariant=\"normal\">{}</mi>", esc(&text)))
            }
            "mathit" => Some(format!("<mi>{}</mi>", esc(self.raw_arg()?))),
            "mathbf" | "bf" | "boldsymbol" | "bm" => markup::styled(self.raw_arg()?, Style::Bold),
            "mathbb" => markup::styled(self.raw_arg()?, Style::Double),
            "mathcal" | "mathscr" => markup::styled(self.raw_arg()?, Style::Script),
            "underline" => Some(format!(
                "<munder accentunder=\"true\">{}<mo>_</mo></munder>",
                self.arg()?
            )),
            "left" => self.fenced(),
            "begin" => self.environment(),
            _ => None,
        }
    }

    /// `\sqrt{body}`, or `\sqrt[index]{body}`.
    fn root(&mut self) -> Option<String> {
        self.skip_space();
        if self.scan.eat('[') {
            let index = self.group(Stop::Bracket)?;
            let body = self.arg()?;
            Some(format!("<mroot>{body}{index}</mroot>"))
        } else {
            Some(format!("<msqrt>{}</msqrt>", self.arg()?))
        }
    }

    /// `\left( ... \right)`, after the `\left`.
    fn fenced(&mut self) -> Option<String> {
        let open = self.fence()?;
        let body = self.group(Stop::Right)?;
        let close = self.fence()?;
        Some(format!("<mrow>{open}{body}{close}</mrow>"))
    }

    /// The delimiter after `\left` or `\right`.
    fn fence(&mut self) -> Option<String> {
        self.skip_space();
        let shown = match self.scan.bump()? {
            '\\' => symbols::fence_command(command_name(&mut self.scan))?,
            ch => symbols::fence_char(ch)?,
        };
        Some(markup::fence(shown))
    }

    /// `\begin{name} ... \end{name}`, after the `\begin`.
    fn environment(&mut self) -> Option<String> {
        let name = self.raw_arg()?;
        let environment = symbols::environment(name)?;
        if environment.columns && self.raw_arg().is_none() {
            return None;
        }
        let rows = self.rows(Stop::Env)?;
        self.skip_space();
        let (end, after) = self.command_ahead()?;
        if end != "end" {
            return None;
        }
        self.scan = after;
        if self.raw_arg()? != name {
            return None;
        }
        Some(format!(
            "<mrow>{}{}{}</mrow>",
            markup::fence(environment.open),
            markup::table(&rows, environment.align),
            markup::fence(environment.close)
        ))
    }
}
