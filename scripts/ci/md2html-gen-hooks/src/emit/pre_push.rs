// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The dispatch block of `_lint_pre_push.sh`: Bucket 1 `spawn_bg` lines,
//! then Bucket 2 `run_seq` lines inside `if (( CODE_CHANGED ))`.

use super::shell::{format_command, format_command_with, indent_block, required_tools_line};
use crate::manifest::{COMMIT_RANGES, Gate, Manifest};

/// Generate the pre-push dispatch block: Bucket 1, then Bucket 2.
///
/// Bucket 1 is `spawn_bg` lines for every pre-push gate with
/// `bucket = "bg"`; Bucket 2 is `run_seq` lines wrapped in
/// `if (( CODE_CHANGED ))` for every gate with `bucket = "seq"`, left out
/// when there is none.  Per-gate special cases (the commit-range gates,
/// vet) are dispatched in [`emit_bg`] / [`emit_seq`].
pub(super) fn render_dispatch(manifest: &Manifest) -> String {
    let gates = manifest.gates_for_tier("pre-push");
    let bg: Vec<&Gate> = gates
        .iter()
        .filter(|gate| gate.bucket.as_deref() == Some("bg"))
        .copied()
        .collect();
    let seq: Vec<&Gate> = gates
        .iter()
        .filter(|gate| gate.bucket.as_deref() == Some("seq"))
        .copied()
        .collect();

    let mut out = String::with_capacity(4 * 1024);
    out.push_str("# ── Dispatch (generated from gates.toml) ──────────────────────────────\n");
    out.push_str("# The tools these gates need beyond the baseline (read by the footer).\n");
    out.push_str(&required_tools_line(&gates));

    out.push_str("# Bucket 1 - fire-and-forget.  Cheap, parallel; no cargo lock\n");
    out.push_str("# contention.  See gates.toml for the canonical gate set.\n");
    for gate in &bg {
        out.push_str(&emit_bg(gate));
    }

    if !seq.is_empty() {
        out.push('\n');
        out.push_str("# Bucket 2 - sequential, fail-fast.  Only runs when code\n");
        out.push_str("# changed (rust | dep | infra).  Pure-docs-only pushes skip\n");
        out.push_str("# the compile/test gate entirely.\n");
        out.push_str("if (( CODE_CHANGED )); then\n");
        for gate in &seq {
            out.push_str(&indent_block(&emit_seq(gate), 4));
        }
        out.push_str("fi\n");
    }

    out
}

/// Bucket 1 emission for a single gate.
///
/// Special cases (in order of precedence):
///   1. A command holding the `{{COMMIT_RANGES}}` placeholder (the commit
///      validators) - the gate's own command, run once per pushed range by a
///      generated function (see [`emit_per_range`]).
///   2. `cargo-vet` + `gate_when="dep_changed"` - emit the `DEP_CHANGED` gate
///      WITH a missing-tool fail and install hint, so a push that changes
///      dependencies cannot skip the audit on a machine without `cargo-vet`.
///   3. Default - unconditional `spawn_bg`.
///
/// Gates whose `gate_when` equals one of `rust_changed` /
/// `infra_changed` / `code_changed` are still emitted unconditionally
/// at Bucket-1: they are cheap enough that running them on a pure-docs
/// push is cheaper than evaluating a guard.
fn emit_bg(gate: &Gate) -> String {
    if gate.command.iter().any(|arg| arg == COMMIT_RANGES) {
        return emit_per_range(gate);
    }

    if gate.tool == "cargo-vet" && gate.when == "dep_changed" {
        return emit_vet(gate);
    }

    format!(
        "spawn_bg \"{label}\" {cmd}\n",
        label = consumer_label(gate),
        cmd = format_command(&gate.command),
    )
}

/// Bucket 2 emission for a single gate.
///
/// Already inside `if (( CODE_CHANGED ))` so the
/// outer rust/code conditionals are implied; an inner `dep_changed`
/// guard is emitted explicitly because `dep_changed` is *strictly
/// stronger* than `code_changed`.
fn emit_seq(gate: &Gate) -> String {
    let cmd = format_command(&gate.command);
    let label = consumer_label(gate);

    if gate.when == "dep_changed" {
        return format!(
            "if (( DEP_CHANGED )); then\n\
             {pad}run_seq \"{label}\" {cmd}\n\
             fi\n",
            label = label,
            cmd = cmd,
            pad = "    ",
        );
    }

    format!("run_seq \"{label}\" {cmd}\n")
}

/// Bucket 1 emission for a gate whose command holds `{{COMMIT_RANGES}}`.
///
/// It is a function that runs the gate's own command once per range in
/// `$COMMIT_RANGES` (one `BASE..OID` per line, from the push's stdin), with the
/// placeholder replaced by that range, stopping at the first failure; then the
/// `spawn_bg` of that function.  When the preamble found no range to
/// check (`$COMMIT_RANGES_UNCHECKED` says why) the gate fails: a gate
/// that checked nothing does not pass.
fn emit_per_range(gate: &Gate) -> String {
    let function = format!("gate_{}", gate.id.replace('-', "_"));
    let cmd = format_command_with(&gate.command, COMMIT_RANGES, "\"$range\"");
    format!(
        "{function}() {{\n\
         {pad}if [[ -n \"$COMMIT_RANGES_UNCHECKED\" ]]; then\n\
         {pad}{pad}printf 'NOT CHECKED: %s\\n' \"$COMMIT_RANGES_UNCHECKED\"\n\
         {pad}{pad}return 1\n\
         {pad}fi\n\
         {pad}local range\n\
         {pad}while IFS= read -r range; do\n\
         {pad}{pad}[[ -z \"$range\" ]] && continue\n\
         {pad}{pad}{cmd} || return\n\
         {pad}done <<< \"$COMMIT_RANGES\"\n\
         }}\n\
         spawn_bg \"{label}\" {function}\n",
        label = consumer_label(gate),
        pad = "    ",
    )
}

/// Hardcoded `vet` block, run when dependencies changed.
///
/// It is the one Bucket-1 gate that uses a non-assumed tool with an install
/// hint, so the audit is not left to CI alone: missing `cargo-vet` on a
/// `dep_changed` push aborts before the rest of the hook runs.
fn emit_vet(gate: &Gate) -> String {
    let cmd = format_command(&gate.command);
    let label = consumer_label(gate);
    format!(
        "if (( DEP_CHANGED )); then\n\
         {pad}if ! command -v {tool} >/dev/null 2>&1; then\n\
         {pad}{pad}printf '%s\u{274c} {tool} required (Cargo.{{toml,lock}} or supply-chain/ changed)%s\\n' \"$C_RED\" \"$C_RESET\" >&2\n\
         {pad}{pad}printf '   %sinstall: %scargo install {tool} --locked%s\\n' \"$C_YELLOW\" \"$C_CYAN\" \"$C_RESET\" >&2\n\
         {pad}{pad}printf '   %sor run:  %sjust install-dev-tools%s\\n'           \"$C_YELLOW\" \"$C_CYAN\" \"$C_RESET\" >&2\n\
         {pad}{pad}exit 2\n\
         {pad}fi\n\
         {pad}spawn_bg \"{label}\" {cmd}\n\
         fi\n",
        tool = gate.tool,
        label = label,
        cmd = cmd,
        pad = "    ",
    )
}

/// Resolve the consumer-side label for a gate at the pre-push tier.
///
/// Honors the per-tier `consumer_names` override (legacy naming) and
/// falls back to the gate id when no override is set.  This keeps
/// the emitted hook byte-equivalent to the pre-Phase-2 hand-written
/// names (`test-build` gate id → `tests` legacy label, etc.).
fn consumer_label(gate: &Gate) -> &str {
    gate.consumer_names
        .get("pre-push")
        .map_or(gate.id.as_str(), String::as_str)
}

#[cfg(test)]
mod tests {
    use super::render_dispatch;
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

    /// Bucket 1 comes before Bucket 2, which sits under `CODE_CHANGED`.
    ///
    /// # Panics
    ///
    /// When the `spawn_bg "fmt"` or the `run_seq "lint-ci"` line is missing,
    /// the second comes first, or there is no `if (( CODE_CHANGED )); then`.
    #[test]
    fn dispatch_emits_bg_then_seq() {
        let manifest = parse(
            r#"
[[gate]]
id="fmt"
label="x"
command=["cargo","fmt","--all","--","--check"]
tiers=["pre-push"]
gate_when="always"
tool="cargo"
bucket="bg"
order=10

[[gate]]
id="lint-ci"
label="x"
command=["just","lint-ci"]
tiers=["pre-push"]
gate_when="rust_changed"
tool="cargo"
bucket="seq"
order=20
"#,
        );
        let out = render_dispatch(&manifest);
        let bg_pos = out.find("spawn_bg \"fmt\"").unwrap();
        let seq_pos = out.find("run_seq \"lint-ci\"").unwrap();
        assert!(bg_pos < seq_pos, "Bucket 1 must be emitted before Bucket 2");
        assert!(out.contains("if (( CODE_CHANGED )); then"));
    }

    /// The two commit validators as the manifest declares them: the same
    /// shape, different scripts.
    const COMMIT_GATES: &str = r#"
[[gate]]
id="commit-subjects"
label="x"
command=["bash","scripts/ci/check_commit_subjects.sh","range","{{COMMIT_RANGES}}"]
tiers=["pre-push"]
gate_when="always"
tool="bash"
bucket="bg"
order=30

[[gate]]
id="commit-signatures"
label="x"
command=["bash","scripts/ci/check_commit_signatures.sh","range","{{COMMIT_RANGES}}"]
tiers=["pre-push"]
gate_when="always"
tool="bash"
bucket="bg"
order=31
"#;

    /// Each commit-range gate's function runs its own validator.
    ///
    /// # Panics
    ///
    /// When there is no `gate_commit_signatures` function; the subject function
    /// does not run `check_commit_subjects.sh` on `"$range"`, or the signature
    /// function `check_commit_signatures.sh`; the signature function runs the
    /// subject validator; a gate is not spawned by its function; or the
    /// `{{COMMIT_RANGES}}` placeholder is left in.
    #[test]
    fn each_commit_range_gate_runs_its_own_command() {
        let out = render_dispatch(&parse(COMMIT_GATES));
        let (subjects, signatures) = out
            .split_once("gate_commit_signatures() {")
            .expect("a function for the signature gate");
        assert!(
            subjects
                .contains("bash scripts/ci/check_commit_subjects.sh range \"$range\" || return"),
            "the subject gate runs the subject validator:\n{out}"
        );
        assert!(
            signatures
                .contains("bash scripts/ci/check_commit_signatures.sh range \"$range\" || return"),
            "the signature gate runs the signature validator:\n{out}"
        );
        assert!(
            !signatures.contains("check_commit_subjects.sh"),
            "the signature gate must not run the subject validator:\n{out}"
        );
        assert!(out.contains("spawn_bg \"commit-subjects\" gate_commit_subjects\n"));
        assert!(out.contains("spawn_bg \"commit-signatures\" gate_commit_signatures\n"));
        // The literal placeholder must NOT leak into the emitted bash.
        assert!(!out.contains("{{COMMIT_RANGES}}"));
    }

    /// Runs the generated functions in bash, with each validator replaced
    /// by a script that records its name and range, over two ranges.
    ///
    /// # Panics
    ///
    /// When the scratch folder or the stub validators cannot be written or
    /// removed, bash cannot be started, the script fails, its output is not
    /// UTF-8, or the validators do not run as subjects on `a..b` and `c..d`,
    /// then signatures on the same, in that order.
    #[test]
    fn commit_range_functions_run_every_range_through_their_validator() {
        let dir = std::env::temp_dir().join(format!("md2html-gen-hooks-{}", std::process::id()));
        let ci = dir.join("scripts/ci");
        std::fs::create_dir_all(&ci).expect("scratch dir");
        for name in ["check_commit_subjects.sh", "check_commit_signatures.sh"] {
            std::fs::write(ci.join(name), format!("printf '%s %s\\n' {name} \"$2\"\n"))
                .expect("stub validator");
        }
        let script = format!(
            "set -euo pipefail\nspawn_bg() {{ shift; \"$@\"; }}\n{}",
            render_dispatch(&parse(COMMIT_GATES))
        );
        let output = std::process::Command::new("bash")
            .args(["-c", &script])
            .current_dir(&dir)
            .env("COMMIT_RANGES", "a..b\nc..d\n")
            .env("COMMIT_RANGES_UNCHECKED", "")
            .output()
            .expect("bash runs");
        std::fs::remove_dir_all(&dir).expect("remove the scratch dir");
        assert!(output.status.success(), "{output:?}");
        let lines: Vec<String> = String::from_utf8(output.stdout)
            .expect("UTF-8 output")
            .lines()
            .filter(|line| line.starts_with("check_"))
            .map(str::to_owned)
            .collect();
        assert_eq!(lines, [
            "check_commit_subjects.sh a..b",
            "check_commit_subjects.sh c..d",
            "check_commit_signatures.sh a..b",
            "check_commit_signatures.sh c..d",
        ]);
    }

    /// With no range to check (no `origin/main`), each commit-range gate
    /// fails and says it checked nothing, instead of passing.
    ///
    /// # Panics
    ///
    /// When bash cannot be started, the script itself fails, its output is not
    /// UTF-8, or each of the two gates does not print `NOT CHECKED:` with the
    /// reason and return 1.
    #[test]
    fn a_commit_range_gate_without_a_range_fails_as_not_checked() {
        let script = format!(
            "set -euo pipefail\nspawn_bg() {{ shift; \"$@\" || echo \"failed: $?\"; }}\n{}",
            render_dispatch(&parse(COMMIT_GATES))
        );
        let output = std::process::Command::new("bash")
            .args(["-c", &script])
            .env("COMMIT_RANGES", "")
            .env(
                "COMMIT_RANGES_UNCHECKED",
                "no origin/main to compare with; fetch it",
            )
            .output()
            .expect("bash runs");
        assert!(output.status.success(), "{output:?}");
        let text = String::from_utf8(output.stdout).expect("UTF-8 output");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines, [
            "NOT CHECKED: no origin/main to compare with; fetch it",
            "failed: 1",
            "NOT CHECKED: no origin/main to compare with; fetch it",
            "failed: 1",
        ]);
    }

    /// Every pre-push gate of the real manifest reaches the hook with its
    /// own command (placeholders replaced): the check that would have
    /// caught a gate running another gate's script.
    ///
    /// # Panics
    ///
    /// When the real `gates.toml` does not parse or validate, or the command of
    /// a pre-push gate, with `{{COMMIT_RANGES}}` replaced by `"$range"`, is not
    /// in the dispatch.
    #[test]
    fn every_real_pre_push_gate_runs_its_own_command() {
        let manifest: Manifest =
            toml::from_str(include_str!("../../../gates.toml")).expect("real manifest");
        manifest.validate().expect("valid manifest");
        let out = render_dispatch(&manifest);
        for gate in manifest.gates_for_tier("pre-push") {
            let expected = super::format_command_with(
                &gate.command,
                crate::manifest::COMMIT_RANGES,
                "\"$range\"",
            );
            assert!(
                out.contains(&expected),
                "gate `{}` does not run `{expected}`:\n{out}",
                gate.id
            );
        }
    }

    /// A missing `cargo-vet` stops the push with an install hint.
    ///
    /// # Panics
    ///
    /// When the dispatch lacks the `DEP_CHANGED` guard, the
    /// `command -v cargo-vet` check, the `install:` line or the
    /// `just install-dev-tools` hint.
    #[test]
    fn vet_emits_dep_changed_hard_fail_block() {
        let manifest = parse(
            r#"
[[gate]]
id="vet"
label="x"
command=["cargo","vet","check","--locked"]
tiers=["pre-push"]
gate_when="dep_changed"
tool="cargo-vet"
bucket="bg"
"#,
        );
        let out = render_dispatch(&manifest);
        assert!(out.contains("if (( DEP_CHANGED )); then"));
        assert!(out.contains("if ! command -v cargo-vet >/dev/null 2>&1"));
        assert!(out.contains("install: "));
        assert!(out.contains("just install-dev-tools"));
    }

    /// A tool outside the baseline is listed as required, never guarded.
    ///
    /// # Panics
    ///
    /// When `typos` is not spawned unconditionally, is guarded by
    /// `command -v typos`, or is not the one entry of `REQUIRED_TOOLS`.
    #[test]
    fn a_tool_outside_the_baseline_is_never_guarded() {
        let manifest = parse(
            r#"
[[gate]]
id="typos"
label="x"
command=["typos","."]
tiers=["pre-push"]
gate_when="always"
tool="typos"
bucket="bg"
"#,
        );
        let out = render_dispatch(&manifest);
        assert!(out.contains("\nspawn_bg \"typos\" typos .\n"));
        assert!(!out.contains("command -v typos"), "{out}");
        assert!(out.contains("REQUIRED_TOOLS=(typos)\n"), "{out}");
    }

    /// A `dep_changed` Bucket 2 gate sits under `DEP_CHANGED`.
    ///
    /// # Panics
    ///
    /// When the indented `if (( DEP_CHANGED )); then` or the `run_seq "deny"`
    /// line is missing.
    #[test]
    fn deny_in_seq_wraps_in_dep_changed() {
        let manifest = parse(
            r#"
[[gate]]
id="deny"
label="x"
command=["cargo","deny","check"]
tiers=["pre-push"]
gate_when="dep_changed"
tool="cargo-deny"
bucket="seq"
"#,
        );
        let out = render_dispatch(&manifest);
        // Bucket 2 emission is indented inside `if (( CODE_CHANGED ))`.
        assert!(out.contains("    if (( DEP_CHANGED )); then"));
        assert!(out.contains("run_seq \"deny\""));
    }

    /// A Bucket 2 gate on `cargo-xwin` runs with no tool guard.
    ///
    /// # Panics
    ///
    /// When the dispatch checks `command -v cargo-xwin`, or lacks the indented
    /// `run_seq "lint-ci-windows"` line.
    #[test]
    fn windows_lint_in_seq_runs_unguarded() {
        let manifest = parse(
            r#"
[[gate]]
id="lint-ci-windows"
label="x"
command=["just","lint-ci-windows"]
tiers=["pre-push"]
gate_when="code_changed"
tool="cargo-xwin"
bucket="seq"
"#,
        );
        let out = render_dispatch(&manifest);
        assert!(!out.contains("command -v cargo-xwin"), "{out}");
        assert!(out.contains("    run_seq \"lint-ci-windows\" just lint-ci-windows\n"));
    }

    /// The pre-push `consumer_names` override labels the emitted gate.
    ///
    /// # Panics
    ///
    /// When `test-build` is not run as `tests`, or is run under its gate id.
    #[test]
    fn consumer_names_override_is_honoured_in_emission() {
        // Mirrors the real `test-build` gate which carries
        // `consumer_names = { "pre-push" = "tests" }` for legacy
        // compatibility with the pre-Phase-2 hook.  The emitted
        // run_seq label MUST use the override; otherwise the drift
        // detector flags the regenerated hook as out of sync with
        // the manifest.
        let manifest = parse(
            r#"
[[gate]]
id="test-build"
label="x"
command=["cargo","nextest","run","--no-run"]
tiers=["pre-push"]
gate_when="code_changed"
tool="cargo-nextest"
bucket="seq"
consumer_names = { "pre-push" = "tests" }
"#,
        );
        let out = render_dispatch(&manifest);
        assert!(
            out.contains("run_seq \"tests\""),
            "expected `run_seq \"tests\"` (override), got:\n{out}"
        );
        assert!(
            !out.contains("run_seq \"test-build\""),
            "raw gate id leaked into output despite override:\n{out}"
        );
    }

    /// Without a pre-push override the gate id is the label.
    ///
    /// # Panics
    ///
    /// When `cargo-check` is not run under its id, or is run under its
    /// `pr-fast` override `sanity`.
    #[test]
    fn missing_consumer_override_falls_back_to_gate_id() {
        // No override for the active tier → emit gate id verbatim.
        let manifest = parse(
            r#"
[[gate]]
id="cargo-check"
label="x"
command=["cargo","check"]
tiers=["pre-push"]
gate_when="code_changed"
tool="cargo"
bucket="seq"
consumer_names = { "pr-fast" = "sanity" }
"#,
        );
        let out = render_dispatch(&manifest);
        // pre-push has no override; emit canonical id.
        assert!(out.contains("run_seq \"cargo-check\""));
        // The pr-fast override must NOT leak into the pre-push emit.
        assert!(!out.contains("run_seq \"sanity\""));
    }

    /// The dispatch is the same for the same manifest, and follows `order`.
    ///
    /// # Panics
    ///
    /// When two renders differ, a `spawn_bg` line is missing, or `b` (order 20)
    /// comes before `a` (order 10).
    #[test]
    fn render_is_deterministic() {
        let toml_text = r#"
[[gate]]
id="b"
label="x"
command=["true"]
tiers=["pre-push"]
gate_when="always"
tool="bash"
bucket="bg"
order=20

[[gate]]
id="a"
label="x"
command=["true"]
tiers=["pre-push"]
gate_when="always"
tool="bash"
bucket="bg"
order=10
"#;
        let first_manifest = parse(toml_text);
        let second_manifest = parse(toml_text);
        let first = render_dispatch(&first_manifest);
        let second = render_dispatch(&second_manifest);
        assert_eq!(first, second);
        // `a` (order=10) before `b` (order=20).
        let at_a = first.find("spawn_bg \"a\"").unwrap();
        let at_b = first.find("spawn_bg \"b\"").unwrap();
        assert!(at_a < at_b, "order 10 before order 20");
    }
}
