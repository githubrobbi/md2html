// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Colour for the text people read: ANSI codes around a piece of text,
//! written only when standard output is a terminal that wants them.
//!
//! The rule is the `CLICOLOR` convention: `CLICOLOR_FORCE` (not `0`)
//! always colours, `NO_COLOR` never does, `CLICOLOR=0` does not, and
//! otherwise colour follows whether standard output is a terminal.

use core::fmt;
use std::ffi::OsStr;
use std::io::IsTerminal as _;
use std::sync::OnceLock;

/// The colours the pipeline paints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Colour {
    /// Failures.
    Red,
    /// Success.
    Green,
    /// Warnings and stages.
    Yellow,
    /// Banners.
    Blue,
    /// Names: steps, branches, versions.
    Cyan,
    /// Separators and pending items.
    Dimmed,
}

impl Colour {
    /// The SGR parameter of the colour.
    const fn code(self) -> &'static str {
        match self {
            Self::Red => "31",
            Self::Green => "32",
            Self::Yellow => "33",
            Self::Blue => "34",
            Self::Cyan => "36",
            Self::Dimmed => "2",
        }
    }
}

/// `text` with a colour (and maybe bold), written with its ANSI codes
/// when [`colour_on`] says so and as plain text otherwise.
pub(crate) struct Painted<'text> {
    /// What is shown.
    text: &'text str,
    /// Its colour.
    colour: Colour,
    /// Whether it is also bold.
    bold: bool,
}

impl Painted<'_> {
    /// The same text, bold as well.
    pub(crate) const fn bold(self) -> Self {
        Self { bold: true, ..self }
    }
}

impl fmt::Display for Painted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !colour_on() {
            return fmt::Display::fmt(self.text, f);
        }
        let weight = if self.bold { "1;" } else { "" };
        write!(
            f,
            "\x1b[{weight}{}m{}\x1b[0m",
            self.colour.code(),
            self.text
        )
    }
}

/// `text` in `colour`, not bold.
const fn painted(text: &str, colour: Colour) -> Painted<'_> {
    Painted {
        text,
        colour,
        bold: false,
    }
}

/// `text` in red: `red("❌")`.
pub(crate) const fn red(text: &str) -> Painted<'_> {
    painted(text, Colour::Red)
}

/// `text` in green: `green("✅")`, `green("done").bold()`.
pub(crate) const fn green(text: &str) -> Painted<'_> {
    painted(text, Colour::Green)
}

/// `text` in yellow.
pub(crate) const fn yellow(text: &str) -> Painted<'_> {
    painted(text, Colour::Yellow)
}

/// `text` in blue.
pub(crate) const fn blue(text: &str) -> Painted<'_> {
    painted(text, Colour::Blue)
}

/// `text` in cyan; a `String` is painted through a borrow, `cyan(&name)`.
pub(crate) const fn cyan(text: &str) -> Painted<'_> {
    painted(text, Colour::Cyan)
}

/// `text` dimmed.
pub(crate) const fn dimmed(text: &str) -> Painted<'_> {
    painted(text, Colour::Dimmed)
}

/// Whether this run writes colour, decided once from the environment and
/// standard output.
fn colour_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        colour_wanted(
            std::env::var_os("CLICOLOR_FORCE").as_deref(),
            std::env::var_os("NO_COLOR").as_deref(),
            std::env::var_os("CLICOLOR").as_deref(),
            std::io::stdout().is_terminal(),
        )
    })
}

/// The `CLICOLOR` convention over the three variables and whether the
/// output is a terminal.
fn colour_wanted(
    force: Option<&OsStr>,
    no_color: Option<&OsStr>,
    clicolor: Option<&OsStr>,
    terminal: bool,
) -> bool {
    if force.is_some_and(|value| value != "0") {
        true
    } else if no_color.is_some() || clicolor.is_some_and(|value| value == "0") {
        false
    } else {
        terminal
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::colour_wanted;

    /// An environment variable's value.
    fn value(text: &str) -> &OsStr {
        OsStr::new(text)
    }

    /// With no variables set, a terminal gets colour and a pipe does not.
    ///
    /// # Panics
    ///
    /// When the terminal gets none or the pipe gets colour.
    #[test]
    fn a_terminal_gets_colour_and_a_pipe_does_not() {
        assert!(colour_wanted(None, None, None, true));
        assert!(!colour_wanted(None, None, None, false));
    }

    /// An empty `NO_COLOR` or `CLICOLOR=0` turns colour off on a terminal.
    ///
    /// # Panics
    ///
    /// When either leaves colour on, or when `CLICOLOR=1` turns it off.
    #[test]
    fn no_color_and_clicolor_zero_turn_it_off() {
        assert!(!colour_wanted(None, Some(value("")), None, true));
        assert!(!colour_wanted(None, None, Some(value("0")), true));
        assert!(colour_wanted(None, None, Some(value("1")), true));
    }

    /// `CLICOLOR_FORCE` gives colour on a pipe, over `NO_COLOR`, unless it is
    /// `0`.
    ///
    /// # Panics
    ///
    /// When `CLICOLOR_FORCE=1` with `NO_COLOR` set gives no colour on a pipe,
    /// or `CLICOLOR_FORCE=0` gives colour.
    #[test]
    fn clicolor_force_wins_unless_it_is_zero() {
        assert!(colour_wanted(
            Some(value("1")),
            Some(value("1")),
            None,
            false
        ));
        assert!(!colour_wanted(Some(value("0")), None, None, false));
    }
}
