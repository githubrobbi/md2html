// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
//
// Hook emission - turns a parsed `Manifest` into the bash text of
// `_lint_pre_push.sh`.
//
// Layout of the emitted file (top to bottom):
//
//   1. AUTO-GENERATED banner + manifest cross-reference.
//   2. Embedded preamble template - colors, change-classification, `spawn_bg` /
//      `run_seq` helpers.  Static; lives in `templates/preamble.sh`. Maintained
//      by hand because it is pure scaffolding (no gate-specific knowledge).
//   3. Generated dispatch block - Bucket 1 (spawn_bg) lines for every pre-push
//      gate with `bucket = "bg"`, then Bucket 2 (run_seq) lines wrapped in the
//      `if (( CODE_CHANGED ))` conditional for every gate with `bucket =
//      "seq"`.  This is the bit the manifest drives.
//   4. Embedded footer template - bucket reaping, result reporting,
//      optional-tool hint, failure dump.  Static; lives in
//      `templates/footer.sh`.
//
// Per-gate special cases are hardcoded (not parameterised by the
// manifest) so the generator stays small and reviewable.  The
// `gates.toml` fields drive the *decision* of which pattern to
// emit; the *templates* themselves live here.

mod fast_conditions;
mod pre_commit;
mod pre_push;
mod shell;

use pre_commit::render_dispatch_fast;
use pre_push::render_dispatch;

use crate::manifest::Manifest;

/// Embedded scaffolding for `_lint_pre_push.sh` - colors,
/// change-classification, `spawn_bg` / `run_seq` helpers.  Pure bash;
/// no per-gate knowledge.
const PREAMBLE_PRE_PUSH: &str = include_str!("../templates/preamble.sh");
/// Embedded scaffolding emitted after `_lint_pre_push.sh`'s dispatch
/// section - bucket reaping, result reporting, optional-tool hint,
/// failure dump.  Pure bash; no per-gate knowledge.
const FOOTER_PRE_PUSH: &str = include_str!("../templates/footer.sh");

/// Embedded scaffolding for `_lint_fast.sh` - colors, staged-file
/// inventory, `has_staged_*` helpers, `spawn` helper.  Pure bash; no
/// per-gate knowledge.
const PREAMBLE_PRE_COMMIT: &str = include_str!("../templates/preamble_fast.sh");

/// Embedded scaffolding emitted after `_lint_fast.sh`'s dispatch
/// section - wait loop, per-job report, optional-tool hint, failure
/// dump.  Pure bash; no per-gate knowledge.
const FOOTER_PRE_COMMIT: &str = include_str!("../templates/footer_fast.sh");

/// Generation target.  One variant per emitted hook file.  Adding a
/// new target means adding a variant here, the matching `render_*`
/// function, and a `templates/preamble_*.sh` + `templates/footer_*.sh`
/// pair.  The `EmitTarget::render` dispatch keeps the rest of the
/// crate target-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmitTarget {
    /// `_lint_pre_push.sh` - the workspace pre-push gate (Phase 2).
    PrePush,
    /// `_lint_fast.sh` - the workspace pre-commit gate (Phase 3a).
    PreCommit,
}

impl EmitTarget {
    /// Render the full bash file as a single owned `String`.
    pub(crate) fn render(self, manifest: &Manifest) -> String {
        match self {
            Self::PrePush => render_pre_push(manifest),
            Self::PreCommit => render_pre_commit(manifest),
        }
    }

    /// Default on-disk path for this target's emitted hook.
    pub(crate) const fn default_output_path(self) -> &'static str {
        match self {
            Self::PrePush => "scripts/hooks/_lint_pre_push.sh",
            Self::PreCommit => "scripts/hooks/_lint_fast.sh",
        }
    }

    /// Manifest-tier name for this target.  Used to filter
    /// `[[gate]]` entries via `Manifest::gates_for_tier`.
    pub(crate) const fn tier(self) -> &'static str {
        match self {
            Self::PrePush => "pre-push",
            Self::PreCommit => "pre-commit",
        }
    }
}

/// Render the complete `_lint_pre_push.sh` file: AUTO-GENERATED
/// banner, embedded preamble, generated dispatch, embedded footer.
fn render_pre_push(manifest: &Manifest) -> String {
    let mut out = String::with_capacity(16 * 1024);
    out.push_str(banner_pre_push());
    out.push_str(PREAMBLE_PRE_PUSH);
    out.push_str(&render_dispatch(manifest));
    out.push_str(FOOTER_PRE_PUSH);
    out
}

/// Render the complete `_lint_fast.sh` file: AUTO-GENERATED banner,
/// embedded preamble, generated dispatch, embedded footer.
fn render_pre_commit(manifest: &Manifest) -> String {
    let mut out = String::with_capacity(8 * 1024);
    out.push_str(banner_pre_commit());
    out.push_str(PREAMBLE_PRE_COMMIT);
    out.push_str(&fast_conditions::helpers(manifest));
    out.push_str(&render_dispatch_fast(manifest));
    out.push_str(FOOTER_PRE_COMMIT);
    out
}

/// Top-of-file banner emitted ahead of the embedded `_lint_pre_push.sh`
/// preamble.  Carries the AUTO-GENERATED notice + a quick-link to the
/// manifest and the regen recipe so a contributor opening the file in
/// their editor knows exactly where to make changes.
// REUSE-IgnoreStart -- the literal `SPDX-License-Identifier:` text
// in the returned bash banner is part of the GENERATED hook's
// header, not this Rust file's REUSE metadata.  Without this
// directive `reuse lint` parses the embedded license id as if it
// were the SPDX expression for `emit.rs` itself and rejects it.
const fn banner_pre_push() -> &'static str {
    "#!/usr/bin/env bash\n\
     # SPDX-License-Identifier: MIT OR Apache-2.0\n\
     # Copyright (c) 2025-2026 Sky, LLC.\n\
     #\n\
     # AUTO-GENERATED by `scripts/ci/md2html-gen-hooks` from `scripts/ci/gates.toml`.\n\
     # MANUAL EDITS WILL BE OVERWRITTEN.\n\
     #\n\
     # To change a gate, edit the manifest and regenerate:\n\
     #     vim scripts/ci/gates.toml\n\
     #     just md2html-gen-hooks\n\
     #\n\
     # Plan: docs/architecture/gates-manifest-plan.md\n\
     #\n\
     # Workspace-wide two-bucket pre-push gate.  See the plan doc for\n\
     # bucket semantics, fail-fast ordering, and the full per-gate\n\
     # rationale (the `notes` field on each `[[gate]]` table in\n\
     # `gates.toml` carries the same documentation that used to live\n\
     # in this header before Phase 2 codegen).\n\
     \n"
}
// REUSE-IgnoreEnd

/// Top-of-file banner emitted ahead of the embedded `_lint_fast.sh`
/// preamble.  Sibling of [`banner_pre_push`]; shape mirrors it
/// exactly, only the regen recipe and the workflow-architecture
/// summary differ.
// REUSE-IgnoreStart -- same rationale as `banner_pre_push`.
const fn banner_pre_commit() -> &'static str {
    "#!/usr/bin/env bash\n\
     # SPDX-License-Identifier: MIT OR Apache-2.0\n\
     # Copyright (c) 2025-2026 Sky, LLC.\n\
     #\n\
     # AUTO-GENERATED by `scripts/ci/md2html-gen-hooks` from `scripts/ci/gates.toml`.\n\
     # MANUAL EDITS WILL BE OVERWRITTEN.\n\
     #\n\
     # To change a gate, edit the manifest and regenerate:\n\
     #     vim scripts/ci/gates.toml\n\
     #     just gen-fast\n\
     #\n\
     # Plan: docs/architecture/gates-manifest-plan.md\n\
     #\n\
     # Staged-scoped parallel pre-commit gate.  See the plan doc for\n\
     # the routing rules and per-gate rationale (the `notes` field on\n\
     # each `[[gate]]` table in `gates.toml` carries the same\n\
     # documentation that used to live in this header before Phase 3a\n\
     # codegen).\n\
     \n"
}
// REUSE-IgnoreEnd

#[cfg(test)]
mod tests {
    use super::EmitTarget;
    use crate::manifest::Manifest;

    /// Parses and validates a manifest fixture.
    fn parse(toml_text: &str) -> Manifest {
        let manifest: Manifest = toml::from_str(toml_text).unwrap();
        manifest.validate().unwrap();
        manifest
    }

    #[test]
    fn full_render_is_idempotent() {
        // Plan §4.4 idempotency contract: running the generator
        // twice in a row must produce no diff on the second run.
        // This covers the entire pipeline (banner + preamble +
        // dispatch + footer), not just the dispatch slice.
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
"#,
        );
        let first = EmitTarget::PrePush.render(&manifest);
        let second = EmitTarget::PrePush.render(&manifest);
        assert_eq!(first, second, "non-deterministic render");
        // Banner + preamble + dispatch + footer must all be present.
        assert!(first.starts_with("#!/usr/bin/env bash\n"));
        assert!(first.contains("AUTO-GENERATED"));
        assert!(first.contains("spawn_bg \"fmt\""));
        assert!(first.contains("BG_FAILED"));
    }

    #[test]
    fn emit_target_default_output_paths_are_distinct() {
        // The drift-detector wires md2html-gen-hooks --check against the
        // matching on-disk path for each target; if both targets
        // accidentally pointed at the same file the second --check
        // would always succeed by reading whatever the first wrote.
        assert_ne!(
            EmitTarget::PrePush.default_output_path(),
            EmitTarget::PreCommit.default_output_path()
        );
        assert_eq!(EmitTarget::PrePush.tier(), "pre-push");
        assert_eq!(EmitTarget::PreCommit.tier(), "pre-commit");
    }
}
