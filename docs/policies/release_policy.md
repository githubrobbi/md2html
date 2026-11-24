<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Versions, the minimum Rust, and how a release is made

For whoever cuts a release, and for whoever depends on md2html across one: what a version number promises, which Rust md2html builds with and when that changes, and the release, step by step, with what to check after it.

## What is versioned

- **One number for the workspace.** `[workspace.package] version` in the root `Cargo.toml`; every crate inherits it (`version.workspace = true`), and the workspace's path dependencies require it, so all move together (lockstep). The product is md2html, one binary; the other crates are its build tooling, never published (`publish = false` for all, `release = false` in `release-plz.toml`), and their number promises nothing.
- **The number is said in three places** that must agree (`AGENTS.md`, section 11): the manifest (with `Cargo.lock`), the capabilities page and its sixteen translations (their `version:` and the `md2html X.Y.Z` they name), and the newest released heading of `CHANGELOG.md`. The gate `versions` (`scripts/ci/versions.py`) holds them together on every push and pull request, and the release workflow checks them again, with the tag, at the commit it releases.
- **A release is a tag `vX.Y.Z` on `main`**, on the merge of the pull request that set the number, with binaries built from it by `.github/workflows/release.yml` (when `LANE_RELEASE` is on). Between releases, `main` carries the last release's number together with the changes under Unreleased: `cargo install --git ... md2html` installs that, and `--tag vX.Y.Z` installs a release.
- **Changes are announced in `CHANGELOG.md`**, in the form of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/): they collect under `## [Unreleased]`, grouped (Security, Added, Changed, Fixed, Development), and the release moves them under `## [X.Y.Z] - date`. A release's notes on GitHub are that section.
- **No features.** md2html has no Cargo features, so none is part of what a version promises; the one feature of the workspace (`build` in `md2html-version`) is the tooling's.

## What a version number promises

md2html is a program: its contract is the command line, written in [command-line.md](../command-line.md) (options and their spellings, settings and their spellings, exit codes and when each is given, environment variables, where files go, which stream carries what, the form of a warning line), and "What is stable" there says what is outside it. A version number says how a release may change that contract.

| A release that | Before 1.0 (today) | From 1.0 |
| --- | --- | --- |
| Fixes what md2html does wrong, or makes it faster, and keeps the contract | patch: `0.7.7` to `0.7.8` | patch |
| Adds to the contract (an option, a setting, a spelling, an environment variable, a warning) | patch | minor |
| Removes or renames any of them, changes an exit code's number or what it means, where a file goes, which stream carries what, or the form of a warning line | minor: `0.7.x` to `0.8.0` | major |
| Raises the minimum Rust | minor | minor |

- A fix may make a run end with the code the contract names for it where it used to end wrongly (a refusal that was missing, a failure that was hidden). That is the contract kept, not changed, and the changelog names it under Security or Fixed.
- `SECURITY.md`'s guarantees hold in every release; a way around one is fixed in the next patch.
- The number of CommonMark examples md2html passes never falls (`spec_tests.rs`).
- The page itself (its markup, classes, styles and scripts), the fragment's markup, the inside of an EPUB and the PDF are not part of the contract and may change in any release; the changelog says when one does in a way a reader of the page notices.
- `just ship` raises the patch number by default; `just ship --bump minor` (or `major`) raises that one. Which one a release needs is read off this table from its Unreleased section.

## Deprecating

md2html has deprecated nothing so far ([MAINTAINING.md](../MAINTAINING.md), section 2). When a part of the contract is to go (an option, a setting, a spelling, an environment variable, an exit code's meaning):

1. A release deprecates it first. It keeps working, and a run that uses it says so in a warning that names what replaces it (a new warning is an addition: a patch release before 1.0, a minor one after). `docs/command-line.md` marks it deprecated, with the version, and `CHANGELOG.md` names it under a `### Deprecated` group.
2. It is removed in a release that raises the minor number (from 1.0, the major), no sooner than three months after the release that deprecated it, and the changelog names it under `### Removed`.
3. A deprecation never holds back a security fix: an option found to break a guarantee of [SECURITY.md](../../SECURITY.md) is fixed, or removed, in the next patch release.

## The minimum Rust

- **What is promised.** md2html builds, with `cargo build` or `cargo install`, on the Rust that `rust-version` in `crates/md2html/Cargo.toml` names, and on every newer stable Rust. Today that is 1.85, the first Rust with the 2024 edition the workspace is written in: md2html cannot be built with anything older, and asks for nothing newer.
- **One number.** `rust-version` in md2html's manifest is the only place it is written for tools; the gate `msrv`, CI's `msrv` job and `just install-dev-tools` read it there. The README, `docs/SUPPLY-CHAIN.md` and this page state it to readers, and the gate `versions` fails when one states another.
- **How it is checked.** The gate `msrv` (every push that changes code) runs `cargo +<rust-version> check --locked -p md2html --all-targets` with `-D warnings`, so md2html and its tests compile on that Rust; CI's `msrv` job runs the same check and the release build a person's `cargo install` makes. Development and every other gate use the pinned nightly of `rust-toolchain.toml`, for rustfmt options that are not yet stable; md2html itself uses nothing unstable.
- **When it may rise.** Only when md2html gains something from a newer Rust (a language feature or a standard-library API the code would otherwise do without), only by the owner's decision (`AGENTS.md`, section 6, gate `msrv`), in a release that raises the minor number, and never past a Rust that has been stable for less than six months. The release names the new minimum under Changed. How: raise `rust-version`, and the statements of the README, `docs/SUPPLY-CHAIN.md` and this page with it (the gate `versions` says where), install that Rust (`just install-dev-tools`), and run `just msrv`.
- **The tooling has none.** The build tooling (`scripts/`, `md2html-version`) is built only with the pinned toolchain; a `rust-version` on it would be a promise nothing checks, and Cargo would hold every crate in the lockfile to the lowest one (the root `Cargo.toml` says so; `md2html-manifest-audit` refuses a `rust-version` anywhere but md2html).

## The release checklist

Releases are switched off today (`LANE_RELEASE` is `false`; `GOTO.md`, section 5). Everything up to the tag works without the lane: a release pull request can be cut, reviewed and merged, and `just release-local` builds the archives on this machine.

### Before

1. `main` is green (`just go`), and the browser suite has run for any change to what a page looks like or does.
2. `CHANGELOG.md`'s Unreleased section is complete and reads as a summary: every change to the contract is in it, and `docs/command-line.md` says the same.
3. The capabilities page's translations are current (`md2html --check docs/capabilities.md docs/capabilities.*.md` prints nothing to fix). A translation left stale stays stale through the release: its fingerprint is renewed only when it was current.
4. Choose the number from the table above.

### Cut the release

5. `just ship` (`--bump minor` or `--bump major` when the table says so; `--skip-toolchain-sync` with `--fresh` keeps the pinned nightly). Phase 1 validates; Phase 2 then raises the number with `cargo set-version` (from `cargo-edit`) and updates `Cargo.lock`, rolls `## [Unreleased]` into `## [X.Y.Z] - today` with its compare link, stamps the capabilities pages (version, date, footer, and each current translation's fingerprint), checks that the three places agree, commits the result signed on `release/vX.Y.Z`, pushes it and opens the pull request.
6. Review the pull request: it changes `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` and the capabilities pages, nothing else. Every required check is green, `versions` among them.
7. `just merge <PR>`.

Without `just ship`, the same by hand, on a branch `release/vX.Y.Z`: `cargo set-version X.Y.Z`; under `## [Unreleased]`, a new `## [X.Y.Z] - YYYY-MM-DD` above the moved entries, the `[Unreleased]` link compared from `vX.Y.Z` and a `[X.Y.Z]` link added; `python3 scripts/ci/versions.py stamp X.Y.Z`; `python3 scripts/ci/versions.py` to confirm; `just go`; commit `chore(release): md2html X.Y.Z`; `just pr`, `just merge`.

### Tag and build (only with `LANE_RELEASE` on)

8. The tag. Two ways are installed, and one release uses one of them:
   - **The workflow's tag** (what `just ship` leads to). The merge of a `release/v*` pull request makes `release-auto-trigger.yml` dispatch `release.yml` for that version at the merge commit; the workflow creates the tag `vX.Y.Z` there through the REST API. That tag is made by the workflow's token and is not signed.
   - **A signed tag.** `just release-tag` on an up-to-date `main` signs `vX.Y.Z` with the maintainer's key and pushes it; the push starts `release.yml`. Use it when the dispatch did not run (the lane was off at the merge, or the branch was not named `release/vX.Y.Z`); when the dispatch ran first, the tag exists and `release-tag` refuses.
9. `release.yml` then: refuses a version whose tag (on a dispatch) or release already exists, a commit not on `main`, and a commit whose three places or tag disagree; builds md2html for Windows x64, macOS on ARM64 and x64, and Linux x64 (static, musl) from nothing but the sources and `Cargo.lock` (no cache: 45 to 75 minutes per target); signs the macOS binaries (Developer ID and notarized when `lane:codesign`'s secrets exist, ad-hoc otherwise); packs each as a ZIP with the licences, README and changelog; writes a CycloneDX SBOM per crate under `crates/`, `CHECKSUMS.txt` and `SHA256SUMS`; signs build provenance when `LANE_SLSA` is on; creates the release with the changelog's section as its notes; and dispatches Homebrew, winget and crates.io when their lanes are on.
10. When it fails, `notify-failure` files an issue. A failure of the runner or the network: re-run the run's failed jobs (the tag step leaves a tag it already made as it is). A failure in the code before the tag exists (a build breaks): fix it on `main`, then dispatch `release.yml` again with the same version and the fixed commit as `commit_sha` (the number has not changed, so the three places still agree); after a signed tag, which already names the old commit, the fix goes out as the next patch. A published release cannot be published again (GitHub's releases are immutable): the fix goes out as the next patch.

### After

11. `gh release view vX.Y.Z` lists the four ZIPs, the four raw binaries, the SBOMs, `CHECKSUMS.txt` and `SHA256SUMS`; the notes are the changelog's section.
12. Download one archive and its checksums: `sha256sum -c CHECKSUMS.txt --ignore-missing` passes, and the binary's `md2html --version` prints `md2html X.Y.Z`. With `LANE_SLSA`, `gh attestation verify <file> --owner githubrobbi` passes.
13. `git fetch --tags`, then `git switch --detach vX.Y.Z` and `python3 scripts/ci/versions.py check --tag vX.Y.Z`: the tag, the manifest, the pages and the changelog agree.
14. `CHANGELOG.md`'s Unreleased section is empty and waits for the next change; `GOTO.md` says what changed in the project's state (the first release, a lane switched on).

### What a rehearsal here proves, and what only a real run does

Without GitHub: the agreement check with a tag (`versions.py check --tag`), a stamp and a roll on a copy of the repository (the gate's test and the pipeline's test of the real changelog), the notes step run in bash, `just release-local` building the archives for six platforms, `cargo package --list`, and actionlint and zizmor on every workflow. Only a real run proves the rest: the runner images and the tools they install (zig and cargo-zigbuild, 7-Zip, codesign, cargo-cyclonedx), the workflow token's permissions (the preflight's draft release), the tag made through the REST API, `gh release create` with both its notes, the SBOMs' names, the attestation, and the dispatches to the packaging lanes.
