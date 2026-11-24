// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html`: Markdown in, one self-contained HTML page out.
//!
//! Forked from iv's `iv-handbook` (same owner, MIT OR Apache-2.0), which
//! itself came from docenta's `handbook-html`. Standard library only: no
//! Markdown crate, no network, nothing to vendor.
//!
//! The Markdown is read the way `CommonMark` reads it, with GitHub's tables,
//! task lists, strikethrough, autolinks, footnotes and alerts, and `$maths$`.
//! The page around it has an outline, search, themes, text size, figures
//! that enlarge, and a glossary whose words explain themselves.
//!
//! The modules stand in layers, from reading text up to the command line;
//! `docs/ARCHITECTURE.md` names them, and `layer_tests` holds the code to
//! them.

extern crate alloc;

mod block;
mod cli;
mod config;
mod document;
mod export;
mod glossary;
mod highlight;
mod inline;
mod input;
mod math;
mod mermaid;
mod model;
mod page;
mod render;
mod report;
mod sanitize;
mod scan;
mod serve;
mod system;
mod text;
mod watch;

/// Runs md2html; the exit code says how it went.
fn main() -> std::process::ExitCode {
    cli::main()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod spec_tests;

#[cfg(test)]
mod layer_tests;
