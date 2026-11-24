<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Versions, the minimum Rust, and how a release is made

For whoever cuts a release, and for whoever depends on md2html across one: what a version number promises, which Rust md2html builds with and when that changes, and the release, step by step, with what to check after it.

## What is versioned

- **One number for the workspace.** `[workspace.package] version` in the root `Cargo.toml`; every crate inherits it (`version.workspace = true`), and the workspace's path dependencies require it, so all move together (lockstep). The product is md2html, one binary; the other crates are its build tooling, never published (`publish = false` for all, `release = false` in `release-plz.toml`), and their number promises nothing.
- **The number is said in three places** that must agree (`AGENTS.md`, section 11): the manifest (with `Cargo.lock`), the capabilities page and its sixteen translations (their `version:` and the `md2html X.Y.Z` they name), and the newest released heading of `CHANGELOG.md`. The gate `versions` (`scripts/ci/versions.py`) holds them together on every push and pull request, and the release workflow checks them again, with the tag, at the commit it releases.
- **A release is a tag `vX.Y.Z` on `main`**, signed by the owner on the merge of the pull request that set the number (`just release-tag`; no workflow makes a tag), with binaries built from it by `.github/workflows/release.yml` (when `LANE_RELEASE` is on). Between releases, `main` carries the last release's number together with the changes under Unreleased: `cargo install --git ... md2html` installs that, and `--tag vX.Y.Z` installs a release. The README and the capabilities pages show the command with the tag of the newest release (decided 2026-11-24): `versions.py stamp` writes it at each release and the gate `versions` holds it to the version.
- **Changes are announced in `CHANGELOG.md`**, in the form of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/): they collect under `## [Unreleased]`, grouped (Security, Added, Changed, Fixed, Development), and the release moves them under `## [X.Y.Z] - date`. A release's notes on GitHub are that section.
- **No features.** md2html has no Cargo features, so none is part of what a version promises; the one feature of the workspace (`build` in `md2html-version`) is the tooling's.

## What a version number promises

md2html is a program: its contract is the command line, written in [command-line.md](../command-line.md) (options and their spellings, settings and their spellings, exit codes and when each is given, environment variables, where files go, which stream carries what, the form of a warning line), together with the rule that gives a heading its anchor, and "What is stable" there says what is outside it. A version number says how a release may change that contract.

| A release that | Before 1.0 (today) | From 1.0 |
| --- | --- | --- |
| Fixes what md2html does wrong, or makes it faster, and keeps the contract | patch: `0.7.7` to `0.7.8` | patch |
| Adds to the contract (an option, a setting, a spelling, an environment variable) | patch | minor |
| Adds a warning, or gives one where it gave none before, so that a document `--check` passed fails it | minor: `0.7.x` to `0.8.0` | minor |
| Removes or renames any of them, changes an exit code's number or what it means, where a file goes, which stream carries what, the form of a warning line, or the rule that gives a heading its anchor | minor | major |
| Raises the minimum Rust | minor | major |

- **A warning raises the minor number, before 1.0 too** (decided 2026-11-24). `--check` exits 1 on any warning, so a build that runs it would turn red on a patch upgrade; a patch release never adds one, whether the warning is new or an old one now given in more documents, and a fix that shows a warning where one was wrongly missing goes out in a release that raises the minor number too. The one exception is security (decided 2026-11-24): a fix that restores a guarantee of [SECURITY.md](../../SECURITY.md) goes out in the next patch even when it adds a warning or gives one in more documents; a `--check` that turns red on it points at a document the guarantee did not hold for.
- **Heading anchors are promised by rule** (decided 2026-11-24). The same heading text gives the same id in every version, so a link to `page.html#installation` keeps working: letters and digits lowercased, letters beyond ASCII kept, any other run of characters one dash, the words of emphasis, code and links counted as text, an explicit `{#id}` taken as written, a heading with no letter or digit named `section`, and an id already on the page (a repeated heading's, or one of the page's own) numbered `-2`, `-3` in order. `crates/md2html/src/tests/anchors.rs` holds the rule with a table of heading texts and their ids, and `crates/md2html/src/render/tests.rs` the list of the page's own ids a heading never takes. Changing the rule breaks links: a minor release before 1.0, a major one from 1.0. md2html's other ids (the bar's, the outline's, a code block's or a diagram's address, a glossary term's) are its own and are not promised.
- **1.0 has no plan yet**: no date and no criteria. Until there is one, the column "Before 1.0" applies.
- A fix may make a run end with the code the contract names for it where it used to end wrongly (a refusal that was missing, a failure that was hidden). That is the contract kept, not changed, and the changelog names it under Security or Fixed.
- `SECURITY.md`'s guarantees hold in every release; a way around one is fixed in the next patch, warnings and all (the exception to the rule above).
- The number of CommonMark examples md2html passes never falls (`spec_tests.rs`).
- The page itself (its markup, classes, styles and scripts, its ids apart from the headings' anchors), the fragment's markup, the inside of an EPUB and the PDF are not part of the contract and may change in any release; the changelog says when one does in a way a reader of the page notices.
- `just ship` raises the patch number by default; `just ship --bump minor` (or `major`) raises that one. Which one a release needs is read off this table from its Unreleased section.

## Deprecating

md2html has deprecated nothing so far ([MAINTAINING.md](../MAINTAINING.md), section 2). When a part of the contract is to go (an option, a setting, a spelling, an environment variable, an exit code's meaning):

1. A release deprecates it first. It keeps working, and a run that uses it says so in a warning that names what replaces it (a new warning: a release that raises the minor number, before 1.0 too). `docs/command-line.md` marks it deprecated, with the version, and `CHANGELOG.md` names it under a `### Deprecated` group.
2. It is removed in a later release that raises the minor number, the next one at the earliest, with no waiting time before 1.0; from 1.0, in a release that raises the major number and no sooner than three months after the release that deprecated it (decided 2026-11-24). The changelog names it under `### Removed`.
3. A deprecation never holds back a security fix: an option found to break a guarantee of [SECURITY.md](../../SECURITY.md) is fixed, or removed, in the next patch release.

## The minimum Rust

- **What is promised.** md2html builds, with `cargo build` or `cargo install`, on the Rust that `rust-version` in `crates/md2html/Cargo.toml` names, and on every newer stable Rust. Today that is 1.85, the first Rust with the 2024 edition the workspace is written in: md2html cannot be built with anything older, and asks for nothing newer.
- **One number.** `rust-version` in md2html's manifest is the only place it is written for tools; the gate `msrv`, CI's `msrv` job and `just install-dev-tools` read it there. The README, `docs/SUPPLY-CHAIN.md` and this page state it to readers, and the gate `versions` fails when one states another.
- **How it is checked.** The gate `msrv` (every push that changes code) runs `cargo +<rust-version> test --locked -p md2html --all-targets` with `-D warnings`, so md2html and its tests compile on that Rust and its tests pass there (a standard library behaves differently from one Rust to another: standard output, `platform_policy.md`); CI's `msrv` job runs the same tests and the release build a person's `cargo install` makes. Development and every other gate use the pinned nightly of `rust-toolchain.toml`, for rustfmt options that are not yet stable; md2html itself uses nothing unstable.
- **When it may rise.** Only when md2html gains something from a newer Rust (a language feature or a standard-library API the code would otherwise do without), only by the owner's decision (`AGENTS.md`, section 6, gate `msrv`), in a release that raises the minor number (the major from 1.0), to any released stable Rust (decided 2026-11-24). The release names the new minimum under Changed. How: raise `rust-version`, and the statements of the README, `docs/SUPPLY-CHAIN.md` and this page with it (the gate `versions` says where), install that Rust (`just install-dev-tools`), and run `just msrv`.
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

8. The tag. Every release tag is signed by the owner (decided 2026-11-24); no workflow makes one. On `main`, `just release-tag` brings it up to date with `origin/main` and refuses unless the commit is release-ready (the three places agree and `CHANGELOG.md` has the version's release heading: `python3 scripts/ci/versions.py check --tag vX.Y.Z`; nothing waits under Unreleased, which would mean `main` moved on after the release pull request), the tag is new here and on GitHub, and `just doctor-signing` passes (a key GitHub would not verify). Then it signs `vX.Y.Z` with the owner's key and pushes it, and the push starts `release.yml`. Only an administrator of the repository may create, move or delete a tag `v*`: the `release-tags` ruleset (`scripts/ci/tag-ruleset.json`, applied with `just protect-main`) refuses anyone else, so a signature GitHub verifies is also the owner's. When changes landed after the release pull request, sign its merge commit by hand (`git tag -s vX.Y.Z <merge-commit> -m "Release vX.Y.Z"`, then `git push origin vX.Y.Z`); the workflow checks it the same way.
9. `release.yml` then: refuses a tag that is not annotated or whose signature GitHub does not verify (the tag object's `verification.verified`, read through the REST API: the signature is good and its key belongs to the GitHub account whose verified email is the tagger's), a version whose release already exists, a commit not on `main`, and a commit whose three places or tag disagree; builds, from the commit the tag names, md2html for Windows x64, macOS on ARM64 and x64, and Linux x64 (static, musl) from nothing but the sources and `Cargo.lock` (no cache: 45 to 75 minutes per target); signs the macOS binaries (Developer ID and notarized when `lane:codesign`'s secrets exist, ad-hoc otherwise); packs each as a ZIP with the licences, README and changelog; writes a CycloneDX SBOM per crate under `crates/`, `CHECKSUMS.txt` and `SHA256SUMS`; signs build provenance when `LANE_SLSA` is on; creates the release on the signed tag with the changelog's section as its notes; and dispatches Homebrew, winget and crates.io when their lanes are on (crates.io only from there, once the release exists: `release-plz.yml` has no tag trigger). Its dispatch takes only the version: it runs the workflow again for a signed tag already there, builds the commit that tag names, and refuses a version with no tag.
10. When it fails, `notify-failure` files an issue. A failure of the runner or the network: re-run the run's failed jobs (a run that can no longer be re-run: dispatch `release.yml` with the same version). A failure in the code (a build breaks): the signed tag names the commit that broke, and a release tag is never moved, so the fix lands on `main` and goes out as the next patch (`just ship`, `just merge`, `just release-tag`); the broken version keeps its changelog section and has no release. A published release cannot be published again (GitHub's releases are immutable): the fix goes out as the next patch.

### After

11. `gh release view vX.Y.Z` lists the four ZIPs, the four raw binaries, the SBOMs, `CHECKSUMS.txt` and `SHA256SUMS`; the notes are the changelog's section.
12. Download one archive and its checksums: `sha256sum -c CHECKSUMS.txt --ignore-missing` passes, and the binary's `md2html --version` prints `md2html X.Y.Z`. With `LANE_SLSA`, `gh attestation verify <file> --owner githubrobbi` passes.
13. `git fetch --tags`, then `git switch --detach vX.Y.Z` and `python3 scripts/ci/versions.py check --tag vX.Y.Z`: the tag, the manifest, the pages and the changelog agree.
14. `CHANGELOG.md`'s Unreleased section is empty and waits for the next change; `GOTO.md` says what changed in the project's state (the first release, a lane switched on).

### What a rehearsal here proves, and what only a real run does

Without GitHub: the agreement check with a tag (`versions.py check --tag`), a stamp and a roll on a copy of the repository (the gate's test and the pipeline's test of the real changelog), the notes step run in bash, `just release-local` building the archives for six platforms, `cargo package --list`, and actionlint and zizmor on every workflow. Only a real run proves the rest: the runner images and the tools they install (zig and cargo-zigbuild, 7-Zip, codesign, cargo-cyclonedx), the workflow token's permissions (the preflight's draft release), the signed tag's verification read through the REST API, `gh release create` with both its notes, the SBOMs' names, the attestation, and the dispatches to the packaging lanes.
