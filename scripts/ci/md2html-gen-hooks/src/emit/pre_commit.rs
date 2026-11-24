// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The dispatch block of `_lint_fast.sh`: staged-scoped parallel `spawn`
//! lines, one per pre-commit-tier gate.

use super::shell::{format_command, format_command_with, required_tools_line};
use crate::manifest::{Gate, Manifest, STAGED_TOML};

/// Generate the dispatch block for `_lint_fast.sh`.
///
/// It holds staged-scoped parallel `spawn` lines, one per pre-commit-tier
/// gate.  Routing is
/// driven by gate id + `gate_when` + the command's placeholders; per-gate
/// special cases (taplo, vet-fmt) are hardcoded mirrors of the existing
/// hand-written hook so behavior is byte-identical at the spawn level.
///
/// Emission order mirrors `Manifest::gates_for_tier` (bucket-rank
/// then `order` then `id`).  All `rust_changed` gates are collapsed into
/// a single `if has_staged_rs || ! has_any_staged; then` block.
pub(super) fn render_dispatch_fast(manifest: &Manifest) -> String {
    let gates = manifest.gates_for_tier("pre-commit");

    // Pre-collect the rust-staged group (fmt / lint-ci / lint-prod /
    // lint-tests, in manifest order).
    let rust_staged: Vec<&Gate> = gates
        .iter()
        .filter(|gate| gate.when == "rust_changed")
        .copied()
        .collect();

    let mut out = String::with_capacity(2 * 1024);
    out.push_str("# ── Dispatch (generated from gates.toml) ──────────────────────────────\n");
    out.push_str("# The tools these gates need beyond the baseline (read by the footer).\n");
    out.push_str(&required_tools_line(&gates));

    let mut emitted_rust_block = false;
    for gate in &gates {
        if gate.command.iter().any(|token| token == STAGED_TOML) {
            out.push_str(&emit_fast_taplo(gate));
            continue;
        }
        if gate.id == "vet-fmt" {
            out.push_str(&emit_fast_vet_fmt(gate));
            continue;
        }
        if gate.when == "rust_changed" {
            if !emitted_rust_block {
                out.push_str(&emit_fast_rust_staged_block(&rust_staged));
                emitted_rust_block = true;
            }
            continue;
        }
        if gate.when == "code_changed" {
            out.push_str("\nif has_staged_code; then\n");
            out.push_str(&emit_fast_default(gate));
            out.push_str("fi\n");
            continue;
        }
        out.push_str(&emit_fast_default(gate));
    }

    out
}

/// Default fast-emit shape: an always-on gate (`file-size`, `typos`,
/// `reuse`), spawned unconditionally.
fn emit_fast_default(gate: &Gate) -> String {
    format!(
        "\nspawn \"{label}\" {cmd}\n",
        label = consumer_label_pre_commit(gate),
        cmd = format_command(&gate.command),
    )
}

/// Fast-emit for the `rust_changed`-tier gates (fmt and the clippy trio).
///
/// They run when Rust is staged or nothing is: a manual `just lint-fast`
/// with nothing staged checks everything rather than passing without
/// having run a lint.  One `if has_staged_rs || ! has_any_staged; then`
/// block holds a `spawn` line for each, in manifest order.  An empty input
/// yields an empty string (no leading newline) so the caller can treat it
/// inline.
fn emit_fast_rust_staged_block(gates: &[&Gate]) -> String {
    if gates.is_empty() {
        return String::new();
    }
    let spawns = gates
        .iter()
        .map(|gate| {
            format!(
                "    spawn \"{label}\" {cmd}",
                label = consumer_label_pre_commit(gate),
                cmd = format_command(&gate.command),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\nif has_staged_rs || ! has_any_staged; then\n{spawns}\nfi\n")
}

/// Fast-emit for `taplo`, the gate whose command holds `{{STAGED_TOML}}`.
///
/// It is the gate's own command with the placeholder replaced by the staged
/// TOML files outside `supply-chain/` (the array `STAGED_TOML_NONVET_FILES`,
/// one path a word), run when such a file is staged.
fn emit_fast_taplo(gate: &Gate) -> String {
    let cmd = format_command_with(
        &gate.command,
        STAGED_TOML,
        "\"${STAGED_TOML_NONVET_FILES[@]}\"",
    );
    format!(
        "\nif has_staged_toml_nonvet; then\n\
         {pad}spawn \"{label}\" {cmd}\n\
         fi\n",
        label = consumer_label_pre_commit(gate),
        pad = "    ",
    )
}

/// Fast-emit for `vet-fmt` - run when supply-chain TOML is staged.
fn emit_fast_vet_fmt(gate: &Gate) -> String {
    format!(
        "\nif has_staged_vet; then\n\
         {pad}spawn \"{label}\" {cmd}\n\
         fi\n",
        label = consumer_label_pre_commit(gate),
        cmd = format_command(&gate.command),
        pad = "    ",
    )
}

/// Resolve the consumer-side label for a gate at the pre-commit tier.
///
/// Honors the per-tier `consumer_names` override (e.g. `fmt` →
/// `fmt-check`) and falls back to the gate id when no override is
/// set.  Sibling of `pre_push::consumer_label`.
fn consumer_label_pre_commit(gate: &Gate) -> &str {
    gate.consumer_names
        .get("pre-commit")
        .map_or(gate.id.as_str(), String::as_str)
}

#[cfg(test)]
mod tests {
    use super::render_dispatch_fast;
    use crate::emit::EmitTarget;
    use crate::manifest::Manifest;

    /// Parses and validates a manifest fixture.
    ///
    /// # Panics
    ///
    /// When the fixture is not TOML of the manifest's shape, or
    /// `Manifest::validate` refuses it.
    fn parse(toml_text: &str) -> Manifest {
        let manifest: Manifest = toml::from_str(&crate::manifest::with_classes(toml_text)).unwrap();
        manifest.validate().unwrap();
        manifest
    }

    /// Fixture covering all four pre-commit dispatch shapes:
    ///   * `fmt`      \u2014 special rust-or-no-staged predicate
    ///   * `file-size`\u2014 always-on
    ///   * `lint-prod`/`lint-tests`/`lint-ci` \u2014 rust-staged group
    ///   * `taplo`    \u2014 special toml-nonvet, the staged files in its
    ///     command
    ///   * `vet-fmt`  \u2014 special vet-staged
    ///   * `typos`    \u2014 always-on
    ///
    /// Mirrors the real manifest's pre-commit-tier subset closely
    /// enough that the generated dispatch matches the on-disk
    /// `_lint_fast.sh` shape; intentionally trimmed (no `reuse`,
    /// no `lint-tests` re-stub) so the assertion message stays
    /// readable when something drifts.
    fn pre_commit_fixture() -> Manifest {
        parse(
            r#"
[[gate]]
id        = "fmt"
label     = "x"
command   = ["cargo", "fmt", "--all", "--", "--check"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "bg"
order     = 10
consumer_names = { "pre-commit" = "fmt-check" }

[[gate]]
id        = "file-size"
label     = "x"
command   = ["bash", "scripts/ci/check_file_size_policy.sh"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "bash"
bucket    = "bg"
order     = 20

[[gate]]
id        = "typos"
label     = "x"
command   = ["typos", "."]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "typos"
bucket    = "bg"
order     = 50

[[gate]]
id        = "taplo"
label     = "x"
command   = ["taplo", "fmt", "--check", "{{STAGED_TOML}}"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "taplo"
bucket    = "bg"
order     = 70

[[gate]]
id        = "vet-fmt"
label     = "x"
command   = ["bash", "scripts/hooks/_check_vet_fmt.sh"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "bash"
bucket    = "bg"
order     = 80

[[gate]]
id        = "lint-prod"
label     = "x"
command   = ["just", "lint-prod"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "seq"
order     = 30

[[gate]]
id        = "lint-tests"
label     = "x"
command   = ["just", "lint-tests"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "seq"
order     = 40

[[gate]]
id        = "lint-ci"
label     = "x"
command   = ["just", "lint-ci"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "seq"
order     = 20
"#,
        )
    }

    /// The fixture's gates reach the dispatch in their six shapes.
    ///
    /// # Panics
    ///
    /// When `fmt` is not spawned as `fmt-check` under its
    /// rust-or-nothing-staged guard; `file-size` or `typos` is not spawned
    /// unconditionally; a gate is guarded by `command -v`; `taplo` is not
    /// spawned on the `STAGED_TOML_NONVET_FILES` array under
    /// `has_staged_toml_nonvet`, carries a shellcheck directive, or the
    /// `{{STAGED_TOML}}` placeholder is left in; `vet-fmt` is not spawned
    /// under `has_staged_vet`; or the rust block is missing or does not spawn
    /// exactly `fmt-check`, `lint-ci`, `lint-prod` and `lint-tests`, in that
    /// order.
    #[test]
    fn fast_dispatch_emits_all_six_shapes() {
        let out = render_dispatch_fast(&pre_commit_fixture());

        // Special-case: fmt's rust-or-no-staged predicate.
        assert!(
            out.contains("if has_staged_rs || ! has_any_staged; then"),
            "missing fmt predicate:\n{out}"
        );
        assert!(out.contains("spawn \"fmt-check\" cargo fmt --all -- --check"));

        // Always-on: file-size unconditional.
        assert!(out.contains("\nspawn \"file-size\" bash scripts/ci/check_file_size_policy.sh\n"));

        // Always-on: typos unconditional, no tool guard (a missing tool
        // fails the gate).
        assert!(out.contains("\nspawn \"typos\" typos .\n"));
        assert!(
            !out.contains("command -v"),
            "no gate is guarded by its tool:\n{out}"
        );

        // Special-case: taplo (toml-nonvet), its own command with the
        // staged files in place of the placeholder.
        assert!(out.contains("\nif has_staged_toml_nonvet; then\n"));
        assert!(!out.contains("shellcheck disable"));
        assert!(
            out.contains("spawn \"taplo\" taplo fmt --check \"${STAGED_TOML_NONVET_FILES[@]}\"\n")
        );
        // The literal `{{STAGED_TOML}}` placeholder must NOT leak.
        assert!(
            !out.contains("{{STAGED_TOML}}"),
            "{{{{STAGED_TOML}}}} placeholder leaked into emitted bash:\n{out}"
        );

        // Special-case: vet-fmt: staged supply-chain TOML is the only
        // condition.
        assert!(out.contains("\nif has_staged_vet; then\n"));
        assert!(out.contains("spawn \"vet-fmt\" bash scripts/hooks/_check_vet_fmt.sh"));

        // Rust-staged group: ONE block, four spawns inside, manifest-order
        // sorted.
        assert!(
            out.contains("if has_staged_rs || ! has_any_staged; then"),
            "missing rust-staged guard:\n{out}"
        );
        let (_, from_block) = out
            .split_once("if has_staged_rs || ! has_any_staged; then\n")
            .expect("rust-staged block");
        let (block, _) = from_block
            .split_once("fi\n")
            .expect("rust-staged block end");
        // fmt (bg) before lint-ci(seq, order=20) before lint-prod(30) before
        // lint-tests(40).
        let spawned: Vec<&str> = block.lines().map(str::trim).collect();
        assert_eq!(
            spawned,
            [
                "spawn \"fmt-check\" cargo fmt --all -- --check",
                "spawn \"lint-ci\" just lint-ci",
                "spawn \"lint-prod\" just lint-prod",
                "spawn \"lint-tests\" just lint-tests",
            ],
            "rust-staged order wrong:\n{block}"
        );
    }

    /// `taplo` and `vet-fmt` run under their staged-file guards.
    ///
    /// # Panics
    ///
    /// When the dispatch lacks the `has_staged_toml_nonvet` guard of `taplo` or
    /// the `has_staged_vet` guard of `vet-fmt`.
    #[test]
    fn staged_gates_run_on_their_staged_files_alone() {
        let manifest = parse(
            r#"
[[gate]]
id        = "taplo"
label     = "x"
command   = ["taplo", "fmt", "--check", "{{STAGED_TOML}}"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "taplo"
bucket    = "bg"

[[gate]]
id        = "vet-fmt"
label     = "x"
command   = ["bash", "scripts/hooks/_check_vet_fmt.sh"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "cargo-vet"
bucket    = "bg"
"#,
        );
        let out = render_dispatch_fast(&manifest);
        assert!(
            out.contains("\nif has_staged_toml_nonvet; then\n"),
            "taplo runs whenever TOML is staged, its tool unguarded:\n{out}"
        );
        assert!(
            out.contains("\nif has_staged_vet; then\n"),
            "vet-fmt runs whenever supply-chain TOML is staged, its tool unguarded:\n{out}"
        );
    }

    /// The `rust_changed` trio is spawned from one `has_staged_rs` block.
    ///
    /// # Panics
    ///
    /// When the dispatch holds no such block, or more than one.
    #[test]
    fn fast_dispatch_emits_exactly_one_rust_staged_block() {
        // Regression-guard for the "emit once at first member" logic
        // in render_dispatch_fast: the rust-staged trio must
        // collapse into ONE `if has_staged_rs; then` block, not
        // three.  A buggy refactor (e.g. losing the
        // `emitted_rust_block` flag) would emit three blocks.
        let out = render_dispatch_fast(&pre_commit_fixture());
        let count = out
            .matches("\nif has_staged_rs || ! has_any_staged; then\n")
            .count();
        assert_eq!(
            count, 1,
            "expected exactly one rust-staged block, found {count}:\n{out}"
        );
    }

    /// `fmt` is spawned under its pre-commit name, `fmt-check`.
    ///
    /// # Panics
    ///
    /// When no spawn is labelled `fmt-check`, or `fmt` is spawned under its
    /// gate id.
    #[test]
    fn fast_dispatch_honours_pre_commit_consumer_override() {
        // `fmt` carries `consumer_names = { "pre-commit" = "fmt-check" }`
        // for legacy compatibility with the pre-Phase-3a hook.  The
        // emitted spawn label MUST use the override; otherwise a
        // pre-commit run would show up as `fmt` (gate id) rather
        // than `fmt-check` (legacy label) and contributors would
        // see a confusing rename.
        let out = render_dispatch_fast(&pre_commit_fixture());
        assert!(
            out.contains("spawn \"fmt-check\""),
            "expected `spawn \"fmt-check\"` (override), got:\n{out}"
        );
        assert!(
            !out.contains("spawn \"fmt\" cargo"),
            "raw gate id leaked into output despite override:\n{out}"
        );
    }

    /// `fmt` also runs when nothing is staged.
    ///
    /// # Panics
    ///
    /// When the guard of `fmt` lacks the `|| ! has_any_staged` branch.
    #[test]
    fn fast_emit_fmt_spans_no_staged_branch() {
        // Property: the `fmt` predicate is a strict superset of
        // every other rust_changed gate.  A no-staged sanity run
        // (manual `just lint-fast` on a clean worktree) MUST still
        // run rustfmt; otherwise the recipe becomes a no-op when
        // the user has nothing staged.
        let manifest = parse(
            r#"
[[gate]]
id        = "fmt"
label     = "x"
command   = ["cargo", "fmt", "--all", "--", "--check"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "bg"
"#,
        );
        let out = render_dispatch_fast(&manifest);
        assert!(
            out.contains("|| ! has_any_staged"),
            "fmt missing no-staged branch:\n{out}"
        );
    }

    /// No `has_staged_rs` block is emitted when `fmt` is the only Rust gate.
    ///
    /// # Panics
    ///
    /// When the guard of `fmt` is missing, or an empty `if has_staged_rs; then`
    /// block is emitted.
    #[test]
    fn fast_emit_rust_staged_block_is_empty_when_no_rust_gates() {
        // Edge case: a pre-commit-tier manifest with NO
        // rust_changed gates other than `fmt` must NOT emit a
        // dangling `if has_staged_rs; then ... fi` block - the
        // generator drops it entirely.
        let manifest = parse(
            r#"
[[gate]]
id        = "fmt"
label     = "x"
command   = ["cargo", "fmt"]
tiers     = ["pre-commit"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "bg"

[[gate]]
id        = "file-size"
label     = "x"
command   = ["bash", "x.sh"]
tiers     = ["pre-commit"]
gate_when = "always"
tool      = "bash"
bucket    = "bg"
"#,
        );
        let out = render_dispatch_fast(&manifest);
        // fmt has its own block; the trio block must NOT exist.
        assert!(out.contains("if has_staged_rs || ! has_any_staged; then"));
        assert!(
            !out.contains("\nif has_staged_rs; then\n"),
            "empty rust-staged block leaked:\n{out}"
        );
    }

    /// Rendering the pre-commit hook twice gives the same complete file.
    ///
    /// # Panics
    ///
    /// When two renders differ, or the file lacks the bash shebang on its first
    /// line, the AUTO-GENERATED notice, the `just gen-fast` recipe,
    /// `set -euo pipefail`, `has_staged_rs`, the dispatch heading, or the
    /// footer's `Required tools` and `✅ lint-fast passed` lines.
    #[test]
    fn pre_commit_render_is_idempotent() {
        // Idempotency contract: running the generator
        // twice in a row must produce no diff on the second run.
        // Regression-guard against any non-deterministic ordering
        // in the gates_for_tier sort or the dispatch emission.
        let manifest = pre_commit_fixture();
        let first = EmitTarget::PreCommit.render(&manifest);
        let second = EmitTarget::PreCommit.render(&manifest);
        assert_eq!(first, second, "non-deterministic pre-commit render");

        // Banner + preamble + dispatch + footer must all be present.
        assert!(first.starts_with("#!/usr/bin/env bash\n"));
        assert!(first.contains("AUTO-GENERATED"));
        assert!(first.contains("just gen-fast"));
        assert!(first.contains("set -euo pipefail"));
        assert!(first.contains("has_staged_rs"));
        assert!(first.contains("# ── Dispatch (generated from gates.toml)"));
        assert!(first.contains("Required tools"));
        assert!(first.contains("✅ lint-fast passed"));
    }

    /// One manifest renders two different files for the two targets.
    ///
    /// # Panics
    ///
    /// When the pre-push and pre-commit files are equal, the pre-commit file
    /// lacks the bare `spawn "file-size"` line, or it holds the pre-push
    /// plumbing `BG_PIDS`, `SEQ_RESULTS` or `CODE_CHANGED`.
    #[test]
    fn pre_commit_and_pre_push_render_distinct_files() {
        // Sanity: the same manifest emits two materially different
        // bash files.  Catches a refactor that accidentally aliases
        // the two render paths (e.g. forgetting to switch templates
        // when adding a new EmitTarget variant).
        let manifest = pre_commit_fixture();
        let pre_push = EmitTarget::PrePush.render(&manifest);
        let pre_commit = EmitTarget::PreCommit.render(&manifest);
        assert_ne!(pre_push, pre_commit);
        // Pre-push uses spawn_bg; pre-commit uses spawn.
        assert!(pre_commit.contains("\nspawn \"file-size\""));
        // Pre-commit must NOT contain the pre-push two-bucket plumbing.
        assert!(
            !pre_commit.contains("BG_PIDS"),
            "pre-push plumbing leaked into pre-commit:\n{pre_commit}"
        );
        assert!(!pre_commit.contains("SEQ_RESULTS"));
        assert!(!pre_commit.contains("CODE_CHANGED"));
    }
}
