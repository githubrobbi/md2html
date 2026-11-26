<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Keeping md2html going

For whoever takes md2html over, or keeps it for a year without the person who wrote it: what to read, what only the owner of the repository can do, and every recurring chore, with how to do it and when it was last done. [GOTO.md](../GOTO.md) says where the project stands today; this page says what keeps it standing.

## 1. Taking it over

Read, in this order: [README.md](../README.md) (what md2html is), [GOTO.md](../GOTO.md) (where it stands, what is switched off, what waits for a decision), [CONTRIBUTING.md](CONTRIBUTING.md) (how a change lands), [AGENTS.md](../AGENTS.md) (the same rules as a checklist, and the fix for every gate), [ARCHITECTURE.md](ARCHITECTURE.md), the policies in [docs/policies/](policies/) ([lint-posture.md](policies/lint-posture.md) first), [TESTING.md](TESTING.md), [SECURITY.md](../SECURITY.md) with [hostile_input_policy.md](policies/hostile_input_policy.md), [SUPPLY-CHAIN.md](SUPPLY-CHAIN.md), and [release_policy.md](policies/release_policy.md). Everything the code promises is in those pages, and a gate or a test holds each promise.

What a new maintainer sets up on their own machine: the tools (`just setup`, then `just install-dev-tools`), their own signing key for this clone (`just setup-signing`, checked by `just doctor-signing`; every commit on `main` is signed, and so is every release tag: `just release-tag` signs it with the same key, and `release.yml` refuses a tag GitHub does not verify), and `gh` signed in with rights on the repository.

What lives outside the repository, on GitHub, and needs an administrator of the repository:

- The `main-protection` ruleset (signed commits, a pull request with every thread resolved, merge commits only, the required checks). It is written in `scripts/ci/main-ruleset.json` and applied with `just protect-main`, which applies the `release-tags` ruleset of `scripts/ci/tag-ruleset.json` too: only an administrator of the repository creates, moves or deletes a tag `v*` (decided 2026-11-24); `bash scripts/ci/bootstrap-github.sh` sets the labels, the merge settings and every `LANE_*` variable.
- The repository variables `LANE_*` (all `false`) and the secrets the dormant lanes would need (`GOTO.md`, section 5). No secret is needed while every lane is off; the workflows use only the token GitHub gives each run.
- Not on GitHub at all: the Developer ID Application certificate for md2html releases and the App Store Connect key that notarizes, in the owner's login keychain (the key as the profile `md2html-notary`), which `just release-sign-macos` uses for a release with `lane:codesign` ([CODE-SIGNING.md](forge/CODE-SIGNING.md), section 5). A new maintainer requests their own from Apple; the old certificate is revoked only if its key may be in other hands (the runbook there), since revoking it stops every binary it signed.
- Security: secret scanning with push protection, Dependabot alerts and security updates, private vulnerability reporting, and CodeQL (its workflow, `codeql.yml`). Reports of a way around a guarantee arrive privately, in the repository's Security tab ([SECURITY.md](../SECURITY.md)).

To hand the project on, give the new maintainer administrator rights on the repository, have them run the three commands above, and check together that `just go` is green on their machine and that `just merge` can merge a pull request of theirs. Nothing else is held by a person: no package registry publishes md2html yet, and no release has been signed by a key other than a maintainer's own.

## 2. Deprecations

md2html has none today: nothing in the command line is kept only for an old user. The other spellings of options (`--output`, `--self-contained`, `--unsafe-html`) and of settings are aliases, not deprecations: [command-line.md](command-line.md) lists them, and they stay. How a part of the contract is retired is written in [release_policy.md](policies/release_policy.md), "Deprecating".

## 3. The chores

Weekly ones run by themselves; a person reads what they report. The rest are done by hand, on a branch, ending with `just go` and a pull request like any change. Each row says when it was last done; update the date in the same pull request.

### Every week, by itself

| What | Where | What a person does | Last |
| --- | --- | --- | --- |
| Dependabot's pull requests for the tooling's crates, the actions and the browser test tools (a week's cooldown) | `.github/dependabot.yml` | Vet each as [SUPPLY-CHAIN.md](SUPPLY-CHAIN.md), chapter 5, says; a cargo bump needs `cargo vet` and `supply-chain/imports.lock` committed to its branch | weekly |
| The deep suite: coverage, Miri, cargo-careful, mutation testing, `cargo udeps` | `tier-2.yml` | A failure files an issue; fix its cause | weekly |
| The audit store's refresh and the nightly-toolchain canary | `cargo-vet-refresh.yml`, `nightly-canary.yml` | The same | weekly |

### Every three months

| What | How | Last |
| --- | --- | --- |
| The toolchain pin | `just toolchain-sync` chooses the newest nightly on which every gate passes for the host, Windows and Linux, and writes it into `rust-toolchain.toml`; then `just go`, and the date in [SUPPLY-CHAIN.md](SUPPLY-CHAIN.md) and `GOTO.md`. The file names the channel and the minimal profile alone: the components and targets a developer's machine gets with the pin are `toolchain_components` and `toolchain_targets` in `just/shared.just` (`just toolchain-ensure`, which `just go` runs), and each CI job adds what it uses (`rustup component add clippy`; gate `workflow-drift`, property 11). `just update-tools` does it together with the gate tools | 2026-11-23 |
| The tooling's crates | `just update-deps` (`cargo update`), then `cargo vet` (an update from a publisher the store does not trust needs its diff read and an audit recorded: `just vet-bump <crate>` prints how), `cargo deny check`, `just go` | 2026-11-23 |
| The supply chain, read | `cargo vet --locked`, `cargo deny check`, `cargo report future-incompatibilities` (no report is the clean answer: it exits 101 with "no reports are currently available"), and the GitHub settings of section 1 read back: `gh api repos/githubrobbi/md2html --jq .security_and_analysis`, `gh api repos/githubrobbi/md2html/private-vulnerability-reporting`, `gh api repos/githubrobbi/md2html/vulnerability-alerts -i` (204 is on), and the open alerts of `code-scanning/alerts`, `dependabot/alerts` and `secret-scanning/alerts` (each should be 0) | 2026-11-24 |

### Every year

| What | Where it is pinned | How | Last |
| --- | --- | --- | --- |
| epubcheck | `EPUBCHECK_VERSION` and `EPUBCHECK_SHA256` in `.github/workflows/ci.yml` (job `epub`); `just install-dev-tools` reads both | Take the W3C's newest release zip from its GitHub releases, compute its SHA-256 (`shasum -a 256`) and compare it with the digest GitHub lists for the asset; write both into `ci.yml`, and the version and the digest into [SUPPLY-CHAIN.md](SUPPLY-CHAIN.md); run `just epubcheck` (every book still passes with no error and no warning) | 2026-11-23 (5.4.0) |
| actionlint | `ACTIONLINT_VERSION` and `ACTIONLINT_SHA256` in `.github/workflows/pr-fast.yml` (job `actionlint`) | The release tarball's SHA-256, checked the same way, into the workflow | 2026-11-24 (1.7.12) |
| zizmor | `ZIZMOR_VERSION` and `ZIZMOR_SHA256` in `.github/workflows/pr-fast.yml` (job `zizmor`) | The release's `zizmor-x86_64-unknown-linux-gnu.tar.gz` (github.com/zizmorcore/zizmor), its SHA-256 computed on the download and compared with the digest GitHub shows for the asset; run the new version over `.github` first, since a new audit may find something to fix | 2026-11-24 (1.33.1) |
| The tools CI installs by version | `scripts/ci/gates.toml`, `[toolchain]` (zig, cargo-zigbuild, and cargo-deny and cargo-vet with the SHA-256 of the Linux x86-64 release each CI job downloads), with the copies in the workflows' `env:` that the gate `workflow-drift` holds to it (it names each stale copy, and refuses an install that names no version); `just install-dev-tools` reads the cargo-deny and cargo-vet versions from the same table; the `tool:` versions of `taiki-e/install-action` in the workflows (shellcheck, typos, taplo, cargo-udeps), which that action checks against its own checksums | Raise each to its newest release that is a week old, in `gates.toml` first and `just md2html-gen-hooks`; zig's SHA-256 for `just install-dev-tools` is read from ziglang.org's published list; a release's SHA-256 for `[toolchain]` is the `digest` GitHub lists for the asset (`gh api repos/<owner>/<repo>/releases/tags/<tag>`), checked against the project's own `.sha256` file; then copy the version and SHA-256 into each workflow `workflow-drift` names; `just go`, then the pull request runs them | 2026-11-24 |
| The runners and the runtimes | `runs-on:` in the workflows: most jobs name `ubuntu-22.04`, a few `ubuntu-24.04` or `macos-14` (the `-latest` ones move by themselves); Node (`node-version`, an LTS) and Java (`java-version`, Temurin, for epubcheck) in `ci.yml` | GitHub retires a runner image some time after its successor arrives, and says when; move each named image to the next before then, and Node to the current LTS, checking that the browser suite and epubcheck still run | 2026-11-24 |
| The minimum Rust | `rust-version` in `crates/md2html/Cargo.toml` | Read [release_policy.md](policies/release_policy.md), "The minimum Rust": it rises only when md2html gains something from a newer Rust, by the owner's decision, in a minor release | 2026-11-24 (1.85) |
| CommonMark | `crates/md2html/tests/commonmark/spec-0.31.2.json` | When the specification has a new version, add its examples beside the old file and point `spec_tests.rs` at them; the number passing may rise, never fall | 2026-11-23 (0.31.2) |
| HTML's named character references | `scripts/entities/entities.json`, the WHATWG's list | Download the WHATWG's `entities.json` over it, run `just entities`, then `just go` (gate `entities-drift`) | 2026-11-23 |
| The characters a heading's anchor keeps | `scripts/unicode/DerivedGeneralCategory.txt`, Unicode's general categories | Only with the minimum Rust, whose standard library's Unicode it follows: download that version's `DerivedGeneralCategory.txt` over it, record its SHA-256 in `scripts/gen_word.py`, run `just word-table`, then `just go` (gate `word-drift`) | 2026-11-25 (16.0.0) |
| The browser test tools | `tests/browser/package.json` and its lockfile | Dependabot proposes them weekly; once a year read [SUPPLY-CHAIN.md](SUPPLY-CHAIN.md), chapter 3, against the versions in the lockfile | 2026-11-22 |
| The rust-forge template | `docs/forge/FORGE-STAMP.toml` | `just forge-upgrade` on a branch (never by copying files) | 2026-11-22 |
| `WINGET_TOKEN` (lane:winget) | the repository's secret: a classic PAT of `githubrobbi`, the owner of the fork `githubrobbi/winget-pkgs`, with `public_repo` and `workflow`, about a year's expiry; a copy in gopass `md2html/github/winget-token` | `winget-token-expiry-check.yml` files an issue four weeks before it expires (and when a fork sync or a branch as komac makes it fails): make a new one with the same two scopes, put it in gopass, `gopass show -o md2html/github/winget-token \| gh secret set WINGET_TOKEN`, dispatch the check, which closes the issue | 2026-11-25 (expires 2027-11-25) |
| The Developer ID Application certificate (lane:codesign) | the owner's login keychain, its SHA-1 in gopass `keys/apple/signing-identity` | Valid five years: some months before it runs out, request a new one for md2html releases (CODE-SIGNING.md, section 5, setup step 1) and put its SHA-1 into gopass; signatures already made keep their validity through Apple's time stamp | not yet (requested before the first signed release) |
| The App Store Connect key that notarizes (lane:codesign) | the keychain profile `md2html-notary` | Does not expire; replace it yearly and on any doubt: a new key with the Developer role, `xcrun notarytool store-credentials md2html-notary` with it, the old key revoked in App Store Connect; after each release, `xcrun notarytool history --keychain-profile md2html-notary` shows no submission you did not make | not yet |
| The translation reviews | the review site, review.4sons.com (its build and deploy in `~/reports/md2html-review`, on the owner's machine), built by `scripts/i18n/review_site.py` | Export what native speakers sent (`build.sh` there names the command: one row per answer, table `answer`), read it with `python3 scripts/i18n/review_answers.py answers.json` (each reviewer's newest answer per question), put each change agreed on into its words file with `python3 scripts/i18n/words.py --set CODE PATH TEXT` or into its page, and a term chosen into `scripts/i18n/terms.json` and every place the language uses it; then `just test`, `--check` on the pages, a pull request naming the reviews it took; rebuild and deploy the site so it shows the new wording | not yet (the site opened 2026-11-26) |
| This page and `GOTO.md` | here | Read both against the repository; a chore that changed gets its row changed | 2026-11-24 |
