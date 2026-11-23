// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Roll `## [Unreleased]` into a dated release section at ship time.
//!
//! `just ship` bumps the lockstep workspace version (see [`crate::version`])
//! and then creates the signed release commit.  Historically that flow never
//! rolled the changelog's `## [Unreleased]` section into a `## [vX.Y.Z]`
//! section, so `## [Unreleased]` silently accumulated already-shipped work and
//! every release in between went unrecorded - the drift repaired wholesale in
//! PR #490.
//!
//! [`roll_changelog_file`] closes that gap as part of the same release commit:
//! it moves the current `## [Unreleased]` body under a fresh dated
//! `## [version]` header, leaves an empty `## [Unreleased]` for the next cycle,
//! and keeps the Keep-a-Changelog footer compare-links correct.  The transform
//! ([`roll_unreleased`]) is pure and unit-tested; the file wrapper is the thin
//! IO shell.

use anyhow::{Context as _, Result};

use crate::child::stdout_text;
use crate::out::say;

/// The canonical unreleased-section header (Keep a Changelog).
const UNRELEASED_HEADER: &str = "## [Unreleased]";

/// Path to the workspace changelog, relative to the repo root that `just ship`
/// runs from.
const CHANGELOG_PATH: &str = "CHANGELOG.md";

/// Roll `CHANGELOG.md` in place: move the `## [Unreleased]` body under a dated
/// `## [version]` section and repoint the footer compare-links.
///
/// Called from Phase 2 of the ship pipeline right after the version bump, so
/// the rolled changelog is staged into the `chore: development vX.Y.Z` release
/// commit.  A missing `CHANGELOG.md` and an empty `## [Unreleased]` are both
/// soft no-ops (the ship flow still proceeds) - only a malformed changelog
/// (no `## [Unreleased]` header at all) is an error.
///
/// # Errors
///
/// Returns an error if the changelog exists but cannot be parsed (no
/// `## [Unreleased]` header) or cannot be written back.
pub(crate) fn roll_changelog_file(version: &str) -> Result<()> {
    let Ok(content) = std::fs::read_to_string(CHANGELOG_PATH) else {
        say!("📝 {CHANGELOG_PATH} not found - skipping changelog roll.")?;
        return Ok(());
    };
    let date = crate::clock::Stamp::now().date();
    // Hand-authored `[Unreleased]` wins. If it is empty, fall back to a draft
    // generated from the commits since the last tag (a git failure is said
    // and means no draft), so a release is never recorded blank.
    if let Some(rolled) = roll_unreleased(&content, version, &date, None)? {
        std::fs::write(CHANGELOG_PATH, rolled)
            .with_context(|| format!("writing rolled {CHANGELOG_PATH}"))?;
        say!("📝 Rolled CHANGELOG [Unreleased] → [{version}] - {date}")?;
        return Ok(());
    }
    let draft = match generate_commit_draft() {
        Ok(draft) => draft,
        Err(error) => {
            say!("📝 No draft from the commits ({error:#}).")?;
            None
        }
    };
    match roll_unreleased(&content, version, &date, draft.as_deref())? {
        Some(rolled) => {
            std::fs::write(CHANGELOG_PATH, rolled)
                .with_context(|| format!("writing rolled {CHANGELOG_PATH}"))?;
            say!(
                "📝 CHANGELOG [Unreleased] was empty - auto-drafted [{version}] notes from \
                 commits (polish anytime; `just changelog-draft` regenerates the draft)."
            )?;
        }
        None => say!(
            "📝 CHANGELOG [Unreleased] is empty and no user-facing commits since the last \
             release - nothing to roll."
        )?,
    }
    Ok(())
}

/// Populate `## [Unreleased]` with a draft from the commits since the last tag.
///
/// This is the on-demand half of the hybrid changelog flow, so there is always
/// a placeholder to polish before shipping (`just changelog-draft`).
///
/// Non-destructive: if `## [Unreleased]` already holds hand-authored notes it
/// is left untouched (the draft never clobbers polish). A missing
/// `CHANGELOG.md` and "no user-facing commits" are soft no-ops.
///
/// # Errors
///
/// Returns an error if the changelog cannot be parsed or written.
pub(crate) fn write_draft_into_unreleased() -> Result<()> {
    let Ok(content) = std::fs::read_to_string(CHANGELOG_PATH) else {
        say!("📝 {CHANGELOG_PATH} not found - nothing to draft.")?;
        return Ok(());
    };
    let Some(draft) = generate_commit_draft()? else {
        say!("📝 No user-facing commits since the last release - nothing to draft.")?;
        return Ok(());
    };
    match inject_unreleased_body(&content, &draft)? {
        Some(updated) => {
            std::fs::write(CHANGELOG_PATH, updated)
                .with_context(|| format!("writing drafted {CHANGELOG_PATH}"))?;
            say!(
                "📝 Wrote an auto-draft into CHANGELOG [Unreleased] - polish it before shipping."
            )?;
        }
        None => say!(
            "📝 CHANGELOG [Unreleased] already has notes - left untouched (a draft never \
             overwrites hand-authored entries)."
        )?,
    }
    Ok(())
}

/// Insert `body` as the `## [Unreleased]` body when that section is empty.
///
/// Returns `Ok(None)` (a signal to leave the file alone) when it already holds
/// content, so a draft never clobbers polish.
///
/// # Errors
///
/// Returns an error if `content` has no `## [Unreleased]` header.
fn inject_unreleased_body(content: &str, body: &str) -> Result<Option<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let unreleased_idx = lines
        .iter()
        .position(|line| line.trim_end() == UNRELEASED_HEADER)
        .context("CHANGELOG.md has no `## [Unreleased]` header")?;
    let body_start = unreleased_idx + 1;
    let next_section_idx = lines
        .iter()
        .enumerate()
        .skip(body_start)
        .find(|(_, line)| line.starts_with("## "))
        .map_or(lines.len(), |(idx, _)| idx);
    let existing: Vec<&str> = lines
        .iter()
        .skip(body_start)
        .take(next_section_idx.saturating_sub(body_start))
        .copied()
        .collect();
    if existing.iter().any(|line| !line.trim().is_empty()) {
        return Ok(None); // already populated - do not overwrite
    }

    let mut out: Vec<String> = Vec::new();
    out.extend(lines.iter().take(body_start).map(|line| (*line).to_owned()));
    out.push(String::new());
    out.extend(body.lines().map(ToOwned::to_owned));
    out.push(String::new());
    out.extend(
        lines
            .iter()
            .skip(next_section_idx)
            .map(|line| (*line).to_owned()),
    );
    let mut joined = out.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    Ok(Some(joined))
}

/// Generate a Keep-a-Changelog draft body from the Conventional-Commit subjects
/// since the last release tag, or `None` when there is nothing user-facing.
///
/// # Errors
///
/// Returns an error if `git log` cannot be run.
pub(crate) fn generate_commit_draft() -> Result<Option<String>> {
    Ok(draft_from_subjects(&commit_subjects_since_last_tag()?))
}

/// Commit subjects (one per line) since the most recent `v*` tag.
///
/// That is the work that a pending release would ship. Falls back to the whole
/// history when no release tag exists yet. Merge commits are excluded: PRs land
/// as merge commits, and each branch commit under one carries its own
/// Conventional-Commit subject.
///
/// # Errors
///
/// Returns an error if `git log` fails.
fn commit_subjects_since_last_tag() -> Result<Vec<String>> {
    use std::process::Command;
    let last_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0", "--match", "v*"])
        .output();
    let range = match last_tag {
        Ok(out) if out.status.success() => {
            format!("{}..HEAD", stdout_text("git describe", out.stdout)?.trim())
        }
        // No release tag yet → summarise the whole history.
        _ => "HEAD".to_owned(),
    };
    let output = Command::new("git")
        .args(["log", "--no-merges", "--format=%s", &range])
        .output()
        .context("running `git log` for the changelog draft")?;
    if !output.status.success() {
        anyhow::bail!("`git log {range}` failed for the changelog draft");
    }
    Ok(stdout_text("git log", output.stdout)?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Turn Conventional-Commit subjects into a grouped Keep-a-Changelog draft.
///
/// Only user-facing types are kept - `feat`→**Added**, `fix`→**Fixed**,
/// `perf`/`refactor`→**Changed**; `ci` / `chore` / `build` / `style` / `docs` /
/// `test` / `revert` are dropped. A `!` breaking marker prefixes the bullet.
/// Subjects arrive newest-first (git-log order) and are reversed to read
/// chronologically. Returns `None` when nothing user-facing remains.
fn draft_from_subjects(subjects: &[String]) -> Option<String> {
    let mut added: Vec<String> = Vec::new();
    let mut changed: Vec<String> = Vec::new();
    let mut fixed: Vec<String> = Vec::new();
    for subject in subjects.iter().rev() {
        let Some((group, bullet)) = classify_subject(subject) else {
            continue;
        };
        let bucket = match group {
            "Added" => &mut added,
            "Changed" => &mut changed,
            _ => &mut fixed,
        };
        if !bucket.contains(&bullet) {
            bucket.push(bullet);
        }
    }
    if added.is_empty() && changed.is_empty() && fixed.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (title, bucket) in [("Added", &added), ("Changed", &changed), ("Fixed", &fixed)] {
        if bucket.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### ");
        out.push_str(title);
        out.push_str("\n\n");
        out.push_str(&bucket.join("\n"));
        out.push('\n');
    }
    Some(out)
}

/// Classify one Conventional-Commit subject into a changelog group + bullet.
///
/// `None` for a non-user-facing (or unparseable) subject. `fix(update): do X`
/// → `("Fixed", "- update: do X")`; a trailing `!` on the type → a
/// `**Breaking:** ` bullet prefix.
fn classify_subject(subject: &str) -> Option<(&'static str, String)> {
    // Split "type(scope)!: description" at the first ": ".
    let (head, raw_description) = subject.split_once(": ")?;
    let description = raw_description.trim();
    if description.is_empty() {
        return None;
    }
    let breaking = head.ends_with('!');
    let type_and_scope = head.strip_suffix('!').unwrap_or(head);
    let (kind, scope) = match type_and_scope.split_once('(') {
        Some((kind, rest)) => (kind, rest.strip_suffix(')')),
        None => (type_and_scope, None),
    };
    let group = match kind.trim() {
        "feat" => "Added",
        "fix" => "Fixed",
        "perf" | "refactor" => "Changed",
        _ => return None,
    };
    let scope_prefix = scope.map_or_else(String::new, |name| format!("{}: ", name.trim()));
    let breaking_prefix = if breaking { "**Breaking:** " } else { "" };
    Some((
        group,
        format!("- {breaking_prefix}{scope_prefix}{description}"),
    ))
}

/// Roll the `## [Unreleased]` section of `content` (a Keep-a-Changelog
/// document) into a dated `## [version] - date` release section.
///
/// Returns `Ok(Some(new_content))` when `## [Unreleased]` held entries to roll,
/// or `Ok(None)` when it was empty (nothing notable to release-note - the
/// caller leaves the file untouched).  `version` is the bumped workspace
/// version without a leading `v` (e.g. `"0.6.16"`); `date` is `YYYY-MM-DD`.
///
/// The transform is re-run safe: the body moves down under the new header and a
/// fresh empty `## [Unreleased]` stays on top, so rolling the result again is a
/// no-op (`Ok(None)`).
///
/// # Errors
///
/// Returns an error if `content` has no `## [Unreleased]` header.
pub(crate) fn roll_unreleased(
    content: &str,
    version: &str,
    date: &str,
    fallback: Option<&str>,
) -> Result<Option<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let unreleased_idx = lines
        .iter()
        .position(|line| line.trim_end() == UNRELEASED_HEADER)
        .context("CHANGELOG.md has no `## [Unreleased]` header to roll")?;
    let body_start = unreleased_idx + 1;

    // The body runs to the next top-level `## ` section (the previous release),
    // or to end-of-document when only the footer link-refs follow.
    let next_section_idx = lines
        .iter()
        .enumerate()
        .skip(body_start)
        .find(|(_, line)| line.starts_with("## "))
        .map_or(lines.len(), |(idx, _)| idx);

    let manual_body: Vec<&str> = lines
        .iter()
        .skip(body_start)
        .take(next_section_idx.saturating_sub(body_start))
        .copied()
        .collect();
    // Prefer the hand-authored `[Unreleased]` body; when it is empty, fall back
    // to a caller-supplied draft (generated from the commits since the last
    // release) so a release is never recorded blank. Empty + no draft is the
    // only true no-op.
    let body: Vec<&str> = if manual_body.iter().all(|line| line.trim().is_empty()) {
        match fallback {
            Some(draft) if !draft.trim().is_empty() => draft.lines().collect(),
            _ => return Ok(None),
        }
    } else {
        manual_body
    };
    let trimmed = trim_blank_edges(&body);

    // Previous release version, parsed from the next `## [x] - ...` header -
    // used for the new footer compare-link.  Absent on a first release.
    let prev_version = lines
        .get(next_section_idx)
        .and_then(|header| parse_section_version(header));

    // Rebuild: prefix (through the [Unreleased] header) / blank / dated header
    // / blank / body / blank / the remaining sections.
    let mut out: Vec<String> = Vec::new();
    out.extend(lines.iter().take(body_start).map(|line| (*line).to_owned()));
    out.push(String::new());
    out.push(format!("## [{version}] - {date}"));
    out.push(String::new());
    out.extend(trimmed.iter().map(|line| (*line).to_owned()));
    out.push(String::new());
    out.extend(
        lines
            .iter()
            .skip(next_section_idx)
            .map(|line| (*line).to_owned()),
    );

    let mut rolled = out.join("\n");
    rolled.push('\n');
    let with_footer = update_footer_links(&rolled, version, prev_version.as_deref());
    Ok(Some(with_footer))
}

/// Drop leading and trailing all-blank lines from a section body, preserving
/// interior blank lines.  The caller has already established that at least one
/// line is non-blank.
fn trim_blank_edges<'body>(body: &[&'body str]) -> Vec<&'body str> {
    let start = body
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(0);
    let end = body
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map_or(0, |idx| idx + 1);
    body.iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .copied()
        .collect()
}

/// Parse the version label out of a `## [x.y.z] - date` (or `## [x.y.z]`)
/// section header, returning `x.y.z` without the surrounding brackets.
fn parse_section_version(header: &str) -> Option<String> {
    let rest = header.strip_prefix("## [")?;
    let end = rest.find(']')?;
    rest.get(..end).map(str::to_owned)
}

/// Repoint the Keep-a-Changelog footer compare-links for the new release:
/// move `[Unreleased]` to `vNEW...HEAD` and add `[vNEW]: …/vPREV...vNEW`.
///
/// Defensive by design: a document with no `[Unreleased]:` link-ref (or a link
/// that does not use the GitHub `/compare/` form) is returned unchanged - the
/// section roll is the load-bearing part, the footer is best-effort polish.
fn update_footer_links(content: &str, version: &str, prev: Option<&str>) -> String {
    let Some(unreleased_link) = content
        .lines()
        .find(|line| line.starts_with("[Unreleased]:"))
    else {
        return content.to_owned();
    };
    // Strip the `[Unreleased]: ` label, then keep the URL up to `/compare/` so
    // the rebuilt links carry only the URL (not a doubled label).
    let Some((_, url)) = unreleased_link.split_once(": ") else {
        return content.to_owned();
    };
    let Some((url_prefix, _)) = url.split_once("/compare/") else {
        return content.to_owned();
    };
    let base = format!("{url_prefix}/compare/");
    let new_unreleased = format!("[Unreleased]: {base}v{version}...HEAD");
    let version_ref_prefix = format!("[{version}]:");
    let already_present = content
        .lines()
        .any(|line| line.starts_with(&version_ref_prefix));
    let new_version_ref =
        prev.map(|prev_ver| format!("[{version}]: {base}v{prev_ver}...v{version}"));

    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("[Unreleased]:") {
            out.push(new_unreleased.clone());
            if let Some(reference) = new_version_ref.as_ref()
                && !already_present
            {
                out.push(reference.clone());
            }
        } else {
            out.push(line.to_owned());
        }
    }
    let mut joined = out.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::{
        classify_subject, draft_from_subjects, inject_unreleased_body, parse_section_version,
        roll_unreleased,
    };

    /// `feat`, `fix` and `perf` map to their groups, with scope and breaking
    /// marker.
    ///
    /// # Panics
    ///
    /// When a `fix`, `feat` or `perf` subject lands in the wrong group or loses
    /// its scope, when `feat(cli)!` lacks the `**Breaking:**` prefix, or when a
    /// `chore`, `ci`, `docs` or non-conventional subject is kept.
    #[test]
    fn classify_subject_maps_types_scopes_and_breaking() {
        assert_eq!(
            classify_subject("fix(update): discover dormant winget installs"),
            Some((
                "Fixed",
                "- update: discover dormant winget installs".to_owned()
            ))
        );
        assert_eq!(
            classify_subject("feat(daemon): per-shard USN loops"),
            Some(("Added", "- daemon: per-shard USN loops".to_owned()))
        );
        assert_eq!(
            classify_subject("perf: faster path resolver"),
            Some(("Changed", "- faster path resolver".to_owned()))
        );
        assert_eq!(
            classify_subject("feat(cli)!: drop --q shorthand"),
            Some((
                "Added",
                "- **Breaking:** cli: drop --q shorthand".to_owned()
            ))
        );
        // Non-user-facing types and unparseable subjects are dropped.
        assert_eq!(classify_subject("chore: bump deps"), None);
        assert_eq!(classify_subject("ci(release): fix musl"), None);
        assert_eq!(classify_subject("docs: tidy readme"), None);
        assert_eq!(classify_subject("not a conventional subject"), None);
    }

    /// The draft groups subjects, drops chores and lists `Added` before
    /// `Fixed`.
    ///
    /// # Panics
    ///
    /// When the draft is `None`, has no `### Added` or `### Fixed` heading,
    /// puts `Fixed` first, lacks either bullet, or keeps the `chore` release
    /// subject.
    #[test]
    fn draft_from_subjects_groups_reverses_and_filters() {
        // git-log order is newest-first; the draft reads chronologically.
        let subjects = vec![
            "chore: release v0.6.23".to_owned(),
            "fix(update): stop double-skip".to_owned(),
            "feat(core): add trigram cache".to_owned(),
        ];
        let draft = draft_from_subjects(&subjects).expect("some user-facing commits");
        let added_at = draft.find("### Added").expect("added section");
        let fixed_at = draft.find("### Fixed").expect("fixed section");
        assert!(added_at < fixed_at, "Added precedes Fixed");
        assert!(draft.contains("- core: add trigram cache"));
        assert!(draft.contains("- update: stop double-skip"));
        assert!(!draft.contains("release v0.6.23"), "chore dropped");
    }

    /// Only `chore` and `ci` subjects give no draft.
    ///
    /// # Panics
    ///
    /// When a draft is made from them.
    #[test]
    fn draft_from_subjects_none_when_nothing_user_facing() {
        let subjects = vec!["chore: x".to_owned(), "ci: y".to_owned()];
        assert!(draft_from_subjects(&subjects).is_none());
    }

    /// An empty `[Unreleased]` is rolled from the fallback draft.
    ///
    /// # Panics
    ///
    /// When the roll errors, when an empty section with a draft is not rolled
    /// into a dated `## [0.6.16]` section holding the draft's bullet, or when a
    /// blank draft is rolled instead of leaving the file alone.
    #[test]
    fn empty_unreleased_rolls_the_fallback_draft() {
        let doc = "# Changelog\n\n## [Unreleased]\n\n## [0.6.15] - 2026-06-28\n\n- x\n";
        let draft = "### Fixed\n\n- update: a thing";
        let out = roll_unreleased(doc, "0.6.16", "2026-06-30", Some(draft))
            .unwrap()
            .expect("rolled from the draft");
        assert!(out.contains("## [0.6.16] - 2026-06-30"));
        assert!(out.contains("- update: a thing"));
        // A blank/absent draft on an empty section is still a no-op.
        assert!(
            roll_unreleased(doc, "0.6.16", "2026-06-30", Some("   "))
                .unwrap()
                .is_none()
        );
    }

    /// A draft fills an empty `[Unreleased]` and leaves a written one alone.
    ///
    /// # Panics
    ///
    /// When the injection errors, when the empty section is not filled with the
    /// draft under its header, or when a section with hand-written notes is
    /// changed.
    #[test]
    fn inject_unreleased_body_fills_empty_but_never_clobbers() {
        let empty = "# Changelog\n\n## [Unreleased]\n\n## [0.6.15] - 2026-06-28\n\n- x\n";
        let filled = inject_unreleased_body(empty, "### Fixed\n\n- y")
            .unwrap()
            .expect("filled the empty section");
        assert!(filled.contains("## [Unreleased]"));
        assert!(filled.contains("- y"));
        // Already-populated → left untouched (draft must not overwrite polish).
        let populated = "## [Unreleased]\n\n### Added\n\n- hand written\n\n## [0.6.15]\n\n- x\n";
        assert!(
            inject_unreleased_body(populated, "### Fixed\n\n- y")
                .unwrap()
                .is_none()
        );
    }

    /// A representative changelog: rich Unreleased body, one prior release, and
    /// Keep-a-Changelog footer compare-links.
    const SAMPLE: &str = "\
# Changelog

## [Unreleased]

### Added - a new thing

- did stuff (#100)

## [0.6.15] - 2026-06-28

### Fixed

- old fix (#1)

[Unreleased]: https://github.com/o/r/compare/v0.6.15...HEAD
[0.6.15]: https://github.com/o/r/compare/v0.6.14...v0.6.15
";

    /// The `[Unreleased]` body moves under a dated section and the links
    /// follow.
    ///
    /// # Panics
    ///
    /// When the roll errors or does nothing, when the dated header or the body
    /// is missing, when the body is still under `[Unreleased]`, or when the
    /// footer links do not point to `v0.6.16...HEAD` and `v0.6.15...v0.6.16`.
    #[test]
    fn rolls_unreleased_into_dated_section() {
        let out = roll_unreleased(SAMPLE, "0.6.16", "2026-06-30", None)
            .unwrap()
            .unwrap();
        assert!(out.contains("## [0.6.16] - 2026-06-30"));
        assert!(out.contains("### Added - a new thing"));
        // The Unreleased header survives but no longer holds the moved body.
        let before_new = out.split("## [0.6.16]").next().unwrap();
        assert!(before_new.contains("## [Unreleased]"));
        assert!(!before_new.contains("new thing"));
        // Footer links repointed.
        assert!(out.contains("[Unreleased]: https://github.com/o/r/compare/v0.6.16...HEAD"));
        assert!(out.contains("[0.6.16]: https://github.com/o/r/compare/v0.6.15...v0.6.16"));
    }

    /// An empty `[Unreleased]` with no draft rolls nothing.
    ///
    /// # Panics
    ///
    /// When the roll errors or returns new content.
    #[test]
    fn empty_unreleased_is_a_noop() {
        let doc = "# Changelog\n\n## [Unreleased]\n\n## [0.6.15] - 2026-06-28\n\n- x\n";
        assert!(
            roll_unreleased(doc, "0.6.16", "2026-06-30", None)
                .unwrap()
                .is_none()
        );
    }

    /// Rolling a rolled changelog again changes nothing.
    ///
    /// # Panics
    ///
    /// When either roll errors, the first rolls nothing, or the second returns
    /// new content.
    #[test]
    fn rolling_twice_is_idempotent() {
        let once = roll_unreleased(SAMPLE, "0.6.16", "2026-06-30", None)
            .unwrap()
            .unwrap();
        assert!(
            roll_unreleased(&once, "0.6.17", "2026-07-01", None)
                .unwrap()
                .is_none()
        );
    }

    /// A changelog with no `## [Unreleased]` header is an error.
    ///
    /// # Panics
    ///
    /// When the roll succeeds on it.
    #[test]
    fn missing_unreleased_header_errors() {
        roll_unreleased("# Changelog\n\n## [0.6.15]\n", "0.6.16", "2026-06-30", None).unwrap_err();
    }

    /// A changelog without footer links is still rolled.
    ///
    /// # Panics
    ///
    /// When the roll errors or does nothing, or the result lacks the dated
    /// header or the moved bullet.
    #[test]
    fn rolls_section_even_without_footer_links() {
        let doc = "## [Unreleased]\n\n- a change\n\n## [0.6.15] - 2026-06-28\n\n- old\n";
        let out = roll_unreleased(doc, "0.6.16", "2026-06-30", None)
            .unwrap()
            .unwrap();
        assert!(out.contains("## [0.6.16] - 2026-06-30"));
        assert!(out.contains("- a change"));
    }

    /// The version label is what sits in a header's brackets.
    ///
    /// # Panics
    ///
    /// When `## [0.6.15] - 2026-06-28` does not give `0.6.15`, `##
    /// [Unreleased]` does not give `Unreleased`, or a line that is no header
    /// gives a label.
    #[test]
    fn parses_section_version_label() {
        assert_eq!(
            parse_section_version("## [0.6.15] - 2026-06-28").as_deref(),
            Some("0.6.15")
        );
        // Returns whatever sits in the brackets verbatim; the roll only ever
        // feeds it a real release header (never the Unreleased one).
        assert_eq!(
            parse_section_version("## [Unreleased]").as_deref(),
            Some("Unreleased")
        );
        assert_eq!(parse_section_version("not a header"), None);
    }
}
