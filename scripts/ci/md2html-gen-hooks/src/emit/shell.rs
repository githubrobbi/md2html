// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Turning a manifest command into bash: which tools the hook names as
//! required, how a token is quoted, how a block is indented.

use alloc::collections::BTreeSet;

use crate::manifest::Gate;

/// The baseline every contributor has once `just setup` ran: tools the
/// hook's `REQUIRED_TOOLS` list leaves out.  Every gate is required
/// either way; the list only decides which tools the footer names when
/// they are missing.
const ASSUMED_TOOLS: &[&str] = &["cargo", "bash", "cargo-nextest", "cargo-deny"];

/// The `REQUIRED_TOOLS=(...)` line the hook footer reads: every tool of
/// `gates` outside the baseline, once each, sorted.
pub(super) fn required_tools_line(gates: &[&Gate]) -> String {
    let tools: BTreeSet<&str> = gates
        .iter()
        .map(|gate| gate.tool.as_str())
        .filter(|tool| !is_assumed(tool))
        .collect();
    let listed = tools
        .into_iter()
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ");
    format!("REQUIRED_TOOLS=({listed})\n")
}

/// Predicate: does the workspace's tooling baseline guarantee that
/// `tool` is on every contributor's PATH?  See [`ASSUMED_TOOLS`].
fn is_assumed(tool: &str) -> bool {
    ASSUMED_TOOLS.contains(&tool)
}

/// Render a TOML command-array as a bash command line.  Tokens are
/// emitted as-is (unquoted) when they are pure shell-safe identifiers
/// or option-style flags, and double-quoted otherwise.  Matches the
/// idiom used in the pre-Phase-2 hand-written hook.
pub(super) fn format_command(cmd: &[String]) -> String {
    cmd.iter()
        .map(|tok| shell_quote(tok))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a TOML command-array as a bash command line in which the one
/// token equal to `placeholder` becomes the shell text `shell` (a
/// variable expansion, written as it stands).  The manifest validator
/// makes sure a placeholder only ever appears as a whole token.
pub(super) fn format_command_with(cmd: &[String], placeholder: &str, shell: &str) -> String {
    cmd.iter()
        .map(|tok| {
            if tok == placeholder {
                shell.to_owned()
            } else {
                shell_quote(tok)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote a single bash token.  Tokens that are pure shell-safe
/// identifiers, paths, flags, or numeric values pass through
/// unquoted; everything else gets single-quoted with embedded
/// single-quotes escaped via the `'\''` idiom.
pub(super) fn shell_quote(token: &str) -> String {
    if token.is_empty() {
        return "''".to_owned();
    }
    let safe = token.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '/' | '.' | ',' | '=' | ':')
    });
    if safe {
        return token.to_owned();
    }
    let escaped = token.replace('\'', r"'\''");
    format!("'{escaped}'")
}

/// Indent every non-empty line of `text` by `n` spaces.  Used to
/// nest Bucket-2 emission inside the `if (( CODE_CHANGED ))` block.
pub(super) fn indent_block(text: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    let mut out = String::with_capacity(text.len() + n * text.lines().count());
    for line in text.lines() {
        if !line.is_empty() {
            out.push_str(&pad);
            out.push_str(line);
        }
        out.push('\n');
    }
    if !text.ends_with('\n') {
        // Preserve trailing-newline absence so the caller's join
        // semantics are not broken.
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn shell_quote_passes_safe_tokens() {
        assert_eq!(shell_quote("cargo"), "cargo");
        assert_eq!(shell_quote("--locked"), "--locked");
        assert_eq!(shell_quote("scripts/ci/x.sh"), "scripts/ci/x.sh");
        assert_eq!(shell_quote(""), "''");
        // Spaces force quoting.
        assert_eq!(shell_quote("a b"), "'a b'");
        // Embedded single quote escaped.
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
