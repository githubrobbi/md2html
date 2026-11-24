// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `git` + `gh` CLI helpers that Phase 2 of the ship pipeline drives.
//!
//! The entry points are [`git_commit`] (create the `chore: development
//! vX.Y.Z ...` signed commit) and [`git_push`] (push the release
//! branch + open the release PR; `just merge <PR>` lands it).  Each of
//! the push sub-steps (`ensure_on_release_branch`, `merge_upstream`,
//! `push_release_branch`, `find_existing_release_pr`, `open_release_pr`,
//! `return_to_base_branch`) is its own small helper
//! so a failure points at a named function in the backtrace and
//! refactors stay surgical.
//!
//! The auto-commit is created on the `release/vX.Y.Z` branch (see
//! [`git_commit`] / `switch_to_release_branch`), never on `main`, so a
//! ship leaves local `main` exactly at `origin/main` - no post-merge
//! `git reset --hard` is ever needed.
//!
//! [`count_unpushed_commits`] makes the push resumable: it lets the ship
//! pipeline detect "HEAD is ahead of
//! `origin/release/<ver>`" and re-run the cached-completed push step
//! instead of silently skipping it.

use std::process::Command;

use anyhow::{Context as _, Result};

use crate::child::{self, captured, stdout_text};
use crate::context::PipelineContext;
use crate::exec::execute_command;
use crate::out::say;
use crate::style::{blue, cyan, green, yellow};
use crate::version::extract_version_from_cargo_toml;

/// Name of the long-lived base branch the release PR targets.
///
/// The ship pipeline never commits onto it locally (see [`git_commit`]); it
/// is only ever the PR base and the branch the developer is left on
/// after a ship.
const BASE_BRANCH: &str = "main";

/// Switch onto the `release/vX.Y.Z` branch, so the auto-commit lands there.
///
/// The branch is created or reset to the current `HEAD`, so the commit
/// lands there instead of on `main`.  Idempotent: safe to re-run on a resumed
/// ship even if the branch already exists.
///
/// Why this matters: committing on `main` left local `main` permanently
/// 1-ahead of `origin/main`, and once the PR merged the local commit
/// diverged from what GitHub recorded on `main`, forcing a
/// `git reset --hard origin/main` after every ship.  Committing on the
/// release branch keeps local `main` exactly at `origin/main`
/// throughout, so a plain `git pull --ff-only` syncs it post-merge.
///
/// `git switch -C` (capital C) creates-or-resets the branch to HEAD,
/// which is exactly the resumable semantics we want: first run creates
/// it at the pre-commit HEAD; a resumed run that is already on the
/// branch (with the commit) re-points it to the same HEAD as a no-op.
///
/// # Errors
///
/// Propagates any failure from the `git switch` subprocess.
fn switch_to_release_branch(ctx: &PipelineContext, release_branch: &str) -> Result<()> {
    say!(
        "🌿 Switching to release branch {} (commit lands here, not on {})",
        cyan(release_branch),
        cyan(BASE_BRANCH)
    )?;
    execute_command(
        "Git switch (release branch)",
        "git",
        &["switch", "-C", release_branch],
        ctx,
    )
}

/// Create the auto-generated `chore: development vX.Y.Z ... [auto-commit]`
/// commit **on the release branch** (never on `main`).  Commit message
/// shape is stable so the release PR template can parse it.
///
/// Switches onto `release/vX.Y.Z` first (via [`switch_to_release_branch`])
/// so `main` is never mutated by the ship - see that helper's docs for
/// the local-`main`-divergence rationale.
///
/// # Errors
///
/// Propagates any failure from the branch switch, the wrapped `git add`,
/// the Cargo.toml read, or the `git commit` subprocess.
pub(crate) fn git_commit(ctx: &PipelineContext) -> Result<()> {
    say!("{}", blue("📝 Creating auto-generated commit..."))?;

    let cargo_toml = std::fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;
    let version = extract_version_from_cargo_toml(&cargo_toml)?;
    let release_branch = format!("release/v{version}");

    // Land the commit on the release branch, not on `main`.
    switch_to_release_branch(ctx, &release_branch)?;

    execute_command("Git add", "git", &["add", "."], ctx)?;
    let commit_message =
        format!("chore: development v{version} - comprehensive testing complete [auto-commit]");
    execute_command("Git commit", "git", &["commit", "-m", &commit_message], ctx)?;
    Ok(())
}

/// Count commits on the local HEAD that have not yet landed on
/// `origin/<remote_branch>`.
///
/// Used by the ship pipeline to decide whether a previously-completed
/// `STEP_GIT_PUSH` needs to be re-run.  After a push succeeds the step
/// is cached; if the developer then commits more locally (e.g. to fix
/// a CI-detected audit failure) and re-runs `just ship`, the cached
/// "completed" state would silently skip the push and the new commits
/// would never land.  Counting `origin/<branch>..HEAD` reliably
/// detects that case.
///
/// Special cases:
/// * If the remote ref does not yet exist (first push of a new release branch),
///   `git rev-list` fails - we treat that as "1 unpushed commit" so the push
///   runs.
/// * If HEAD equals the remote ref, the count is 0 and the cached completion is
///   honoured (idempotent re-runs stay cheap).
///
/// # Errors
///
/// Returns an error when the `git rev-list` subprocess cannot be spawned
/// or prints something other than a count (non-UTF-8 output included).
/// A non-zero exit status is treated as the "ref missing" special case
/// and folded into `Ok(1)`.
pub(crate) fn count_unpushed_commits(remote_branch: &str) -> Result<u64> {
    let remote_ref = format!("origin/{remote_branch}");
    let spec = format!("{remote_ref}..HEAD");
    let out = captured(
        Command::new("git").args(["rev-list", "--count", &spec]),
        child::LOCAL,
    )
    .with_context(|| format!("Failed to run git rev-list for {spec}"))?;
    if !out.status.success() {
        // Remote ref doesn't exist yet (first push) - be conservative
        // and treat HEAD as ahead so the push always runs.
        return Ok(1);
    }
    let text = stdout_text("git rev-list --count", out.stdout)?;
    text.trim().parse::<u64>().with_context(|| {
        format!(
            "`git rev-list --count {spec}` printed `{}`, not a count",
            text.trim()
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// `git_push` sub-steps
// ─────────────────────────────────────────────────────────────────────────────

/// Ensure the working tree is on the existing `release_branch` before
/// pushing - without resetting it.
///
/// Uses plain `git switch <branch>` (NOT `-C`): on a resumed ship where
/// `git_commit` was cached-skipped, the release branch already exists
/// with the auto-commit and we must land on *that* commit, not reset the
/// branch to a commit-less `HEAD`.  On a fresh run we are already on the
/// release branch (just created by `git_commit`), so the switch is a
/// no-op.  `git switch` to the already-current branch succeeds silently.
///
/// # Errors
///
/// Propagates any failure from the `git switch` subprocess (e.g. the
/// branch does not exist - which would indicate `git_commit` never ran).
fn ensure_on_release_branch(ctx: &PipelineContext, release_branch: &str) -> Result<()> {
    execute_command(
        "Git switch (ensure release branch)",
        "git",
        &["switch", release_branch],
        ctx,
    )
}

/// Merge `origin/<BASE_BRANCH>` into the currently-checked-out release
/// branch so the PR is based on current mainline before it opens.
///
/// A MERGE, never a rebase: the branch's commits are signed, and a PR
/// lands on `main` as a merge commit with those commits kept one by
/// one.  A rebase would rewrite (and drop the signatures of) every
/// commit, and flatten any merge already on the branch.  When the
/// branch already contains `origin/main` this is a no-op.
///
/// # Errors
///
/// Propagates any failure from the wrapped `git pull --no-rebase`
/// subprocess (network, merge conflicts, ...).
fn merge_upstream(ctx: &PipelineContext) -> Result<()> {
    execute_command(
        "Git pull merge",
        "git",
        &["pull", "--no-rebase", "--no-edit", "origin", BASE_BRANCH],
        ctx,
    )
}

/// Push `HEAD` to `refs/heads/<release_branch>` on `origin`.  No-op /
/// fast-forward when the pipeline is resuming after a previously-
/// failed step 11 that already pushed the same commit.
///
/// # Errors
///
/// Propagates any failure from the wrapped `git push` subprocess.
fn push_release_branch(ctx: &PipelineContext, release_branch: &str) -> Result<()> {
    let push_ref = format!("HEAD:refs/heads/{release_branch}");
    say!("📤 Pushing HEAD to {}", cyan(release_branch))?;
    execute_command(
        "Git push (release branch)",
        "git",
        &["push", "origin", &push_ref],
        ctx,
    )
}

/// Return the PR number of an already-open PR for `release_branch`, or
/// `None` if no open PR exists.  Used by [`git_push`] to keep the
/// PR-creation step idempotent across resumed ship runs.
///
/// # Errors
///
/// `gh pr list` cannot be spawned, fails (its error output is in the
/// message), or prints something that is not UTF-8.
fn find_existing_release_pr(release_branch: &str) -> Result<Option<String>> {
    let existing_pr_output = captured(
        Command::new("gh").args([
            "pr",
            "list",
            "--head",
            release_branch,
            "--state",
            "open",
            "--json",
            "number",
            "-q",
            ".[0].number",
        ]),
        child::NETWORK,
    )
    .context("Failed to query existing release PR via gh")?;
    if !existing_pr_output.status.success() {
        anyhow::bail!(
            "`gh pr list --head {release_branch}` failed ({}): {}",
            existing_pr_output.status,
            stdout_text("gh pr list (its error output)", existing_pr_output.stderr)?.trim()
        );
    }
    let trimmed = stdout_text("gh pr list", existing_pr_output.stdout)?
        .trim()
        .to_owned();
    Ok((!trimmed.is_empty()).then_some(trimmed))
}

/// The body of the release PR.
///
/// It says what the PR changes, how it lands (`just merge`, a merge
/// commit), and the step after the merge (`just release-tag`, whose
/// signed tag's push starts `release.yml`; nothing starts by itself).
fn release_pr_body(base_branch: &str, release_branch: &str, version: &str) -> String {
    format!(
        "## Summary\n\n\
         `just ship` Phase 2 auto-commit for **v{version}** - the \
         `[workspace.package].version` bump in `Cargo.toml` and the \
         rolled CHANGELOG.\n\n\
         ## Merge\n\n\
         Land it with `just merge <PR>`: the recipe watches every check \
         to completion and merges with a merge commit, the branch's \
         signed commits kept one by one (never squash, never rebase).  \
         No auto-merge is queued: on a plan without required checks an \
         auto-merge fires before a single check has run.\n\n\
         ## After merge\n\n\
         Nothing starts by itself: every release tag is signed by the \
         owner.  Bring `{base_branch}` up to date with \
         `git pull --ff-only origin {base_branch}` (the auto-commit lived \
         only on `{release_branch}`, so local `{base_branch}` never \
         drifted), then run `just release-tag` there: it checks that \
         `{base_branch}` is release-ready for v{version}, signs the tag \
         `v{version}` and pushes it, and the push starts `release.yml`, \
         which builds the binaries and publishes GitHub Release \
         v{version}."
    )
}

/// Open a new release PR via `gh pr create`, with a title + body that
/// explain how it lands and what follows the merge
/// ([`release_pr_body`]).
///
/// # Errors
///
/// Propagates any failure from the wrapped `gh pr create` subprocess.
fn open_release_pr(
    ctx: &PipelineContext,
    base_branch: &str,
    release_branch: &str,
    version: &str,
) -> Result<()> {
    let pr_title = format!("chore: release v{version} - ship pipeline auto-commit");
    let pr_body = release_pr_body(base_branch, release_branch, version);

    say!("📬 Opening release PR")?;
    execute_command(
        "Open release PR",
        "gh",
        &[
            "pr",
            "create",
            "--base",
            base_branch,
            "--head",
            release_branch,
            "--title",
            &pr_title,
            "--body",
            &pr_body,
        ],
        ctx,
    )
}

/// Push the release branch and open the release PR against
/// [`BASE_BRANCH`].  Branch-protection compatible: never pushes
/// directly to `main`.
///
/// Thin orchestrator - each sub-step lives in its own helper
/// ([`ensure_on_release_branch`], [`merge_upstream`],
/// [`push_release_branch`], [`find_existing_release_pr`],
/// [`open_release_pr`], [`return_to_base_branch`])
/// so the control flow stays readable and individual failures map 1:1
/// to a named helper in the backtrace.
///
/// # Errors
///
/// Propagates any failure from the helper subprocesses.  The version
/// lookup via [`crate::version::get_current_version`] is the one
/// non-subprocess error source.
pub(crate) fn git_push(ctx: &PipelineContext) -> Result<()> {
    say!(
        "{}",
        blue("🚀 Opening release PR (branch-protection-compatible)...")
    )?;

    // The auto-commit was made on the release branch by `git_commit`, so
    // that is the branch checked out here.  Derive its name from the
    // version Phase 2 step 07 bumped (example: `release/v0.5.68`).
    let version = crate::version::get_current_version()?;
    let release_branch = format!("release/v{version}");
    say!("📌 Release branch: {}", cyan(&release_branch))?;

    // Resilience for resumed ships: if step 10 (git_commit) was
    // cached-complete this run, we were NOT switched onto the release
    // branch - and a prior git_push may have already switched us back to
    // `main`.  Re-checkout the existing release branch (plain `switch`,
    // NOT `-C`, so we land on the branch that already holds the
    // auto-commit rather than resetting it to a commit-less HEAD).
    ensure_on_release_branch(ctx, &release_branch)?;

    // Merge current mainline into the release branch before opening the
    // PR (keeps the PR based on the latest `origin/main`, history intact).
    merge_upstream(ctx)?;

    push_release_branch(ctx, &release_branch)?;

    // Idempotent PR creation: reuse an existing open PR for the same
    // release branch if the pipeline is resuming from a previously-
    // failed step 11.  Base is always `main` - the ship never targets
    // any other branch.
    let pr_number = if let Some(pr_number) = find_existing_release_pr(&release_branch)? {
        say!("ℹ️  Reusing existing release PR #{}", cyan(&pr_number))?;
        Some(pr_number)
    } else {
        open_release_pr(ctx, BASE_BRANCH, &release_branch, &version)?;
        find_existing_release_pr(&release_branch)?
    };

    // Leave the developer back on `main`, untouched and exactly at
    // `origin/main`.  Because the commit only ever lived on the release
    // branch, local `main` never drifted - after the PR merges, a plain
    // `git pull --ff-only` syncs it (no `reset --hard` needed).
    return_to_base_branch(ctx)?;

    say!("{} Release PR for v{} opened", green("✅"), version)?;
    let pr_ref = pr_number.unwrap_or_else(|| release_branch.clone());
    say!(
        "   💡 Land it:      {}",
        cyan(&format!(
            "just merge {pr_ref}  (watches every check, then a merge commit)"
        ))
    )?;
    say!(
        "   💡 After merge:  {}",
        cyan(&format!(
            "git pull --ff-only origin {BASE_BRANCH} (local {BASE_BRANCH} never drifted), then"
        ))
    )?;
    say!(
        "   💡 Then:         {}",
        cyan("just release-tag  (signs the tag; its push starts release.yml)")
    )?;

    Ok(())
}

/// Switch the working tree back to [`BASE_BRANCH`] after the release PR
/// has been opened, so the developer is left where they started.
///
/// Best-effort: a failure here does not undo the already-opened PR, so
/// it is logged but not fatal - the ship's real deliverable (the
/// release PR) is already done by the time this runs.
///
/// # Errors
///
/// Never returns `Err` - a failed switch is downgraded to a warning so
/// it cannot fail an otherwise-successful ship.
fn return_to_base_branch(ctx: &PipelineContext) -> Result<()> {
    say!("🔙 Returning to {}", cyan(BASE_BRANCH))?;
    if let Err(err) = execute_command("Git switch (base)", "git", &["switch", BASE_BRANCH], ctx) {
        say!(
            "{} could not switch back to {BASE_BRANCH} ({err}); you are still on the release branch",
            yellow("⚠️")
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::release_pr_body;

    /// The release PR says how it lands and what follows the merge: `just
    /// merge`, then `just release-tag` on an up-to-date base branch, whose
    /// push starts `release.yml`; it names no workflow that starts by
    /// itself.
    ///
    /// # Panics
    ///
    /// When the body leaves out `just merge`, `just release-tag`, the pull
    /// that brings the base branch up to date, `release.yml` or the tag
    /// `v1.2.3`, or names `release-auto-trigger`.
    #[test]
    fn the_release_pr_says_the_signed_tag_comes_after_the_merge() {
        let body = release_pr_body("main", "release/v1.2.3", "1.2.3");
        for needed in [
            "just merge <PR>",
            "just release-tag",
            "git pull --ff-only origin main",
            "`release.yml`",
            "`v1.2.3`",
        ] {
            assert!(body.contains(needed), "the body names {needed}: {body}");
        }
        assert!(
            !body.contains("release-auto-trigger"),
            "no workflow starts a release by itself: {body}"
        );
        let merge = body.find("just merge").expect("the body names just merge");
        let tag = body
            .find("just release-tag")
            .expect("the body names just release-tag");
        assert!(merge < tag, "the merge comes before the tag: {body}");
    }
}
