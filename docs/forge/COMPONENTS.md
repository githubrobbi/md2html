<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# COMPONENTS.md: the growth catalog

The master document for growing md2html's workspace, for whoever is asked
to switch on a dormant capability or add structure; the template md2html
adopted ships the same catalog. Every lane is the owner's decision. Two
kinds of entries share one schema:

- **`lane:*`**: machinery that is ALREADY INSTALLED and dormant. Enabling a
  lane is a data change (repo variable, TOML flag, gate tier), never a new
  file. Everything a lane needs is already in the repository.
- **`component:*`**: structure that is NOT yet installed. Adding a component
  creates files (a crate, a fuzz target, a bench) following the documented
  recipe.

Every entry ends with a **verify** command; a recipe that leaves the machine
inconsistent fails there, loudly. Rule of review: **dormancy is a data change;
growth is a recipe. Never delete scaffolding to "simplify".**

---

## lane:release, GitHub release binaries

- **what:** `release.yml` builds the workspace binaries for the 4-target
  matrix (Windows x64, macOS arm64 and Intel, Linux x64 musl), packages
  archives + SHA256SUMS, creates the GitHub release.
  `release-auto-trigger.yml` dispatches it when a `release/v*` PR merges;
  `release-cache-warm.yml` keeps builders warm.
- **prerequisites:** none (first lane most projects enable)
- **tooling:** none beyond GitHub
- **secrets/vars:** set repo variable `LANE_RELEASE=true`
- **touches:** nothing; the guard is `if: vars.LANE_RELEASE == 'true'` on
  every job. Growing the binary list: extend the commented list in
  `release.yml` (marked "Add additional workspace binaries here").
- **runbook:** `gh variable set LANE_RELEASE --body true`; then `just ship`
  and merge the release PR; the tag build fires automatically.
- **verify:** `just release-status` after the first merge.

## lane:release-plz, publishing to crates.io

- **what:** `release-plz.yml`, dispatched by hand for a release tag,
  publishes the crates whose block in `release-plz.toml` says
  `release = true`. It neither bumps versions nor writes the changelog:
  md2html's changelog is written by hand and rolled by `just ship`, and the
  tag `v<version>` is the release.
- **prerequisites:** a crate worth publishing (today none is: md2html is
  installed with `cargo install --git`), and this repository made its
  trusted publisher on crates.io: the publish job signs in with OIDC
  (`rust-lang/crates-io-auth-action`), so no token is stored.
- **secrets/vars:** `LANE_RELEASE_PLZ=true` and `ENABLE_CRATES_IO_PUBLISH=true`
- **verify:** dispatch `release-plz.yml` with `publish_tag` set to a release
  tag; the run shows the `crates-io-publish` job.

## lane:crates-publish, crates.io publishing

- **what:** publish selected leaf crates to crates.io; weekly
  `crates-io-dry-run.yml` (`cargo publish --dry-run` + `cargo-semver-checks`)
  guards publishability continuously.
- **prerequisites:** crate names reserved on crates.io; crate has its own
  `README.md`, tailored `keywords`/`categories`. The manifest audit refuses
  those overrides unless the crate is named in its lists for this lane
  (`scripts/ci/md2html-manifest-audit/src/audit.rs`, `KnownExceptions::new`,
  empty today); adding a name there is part of the owner's decision to
  publish, with the reason beside it.
- **tooling:** `cargo login` locally once
- **secrets/vars:** no token (OIDC, as `lane:release-plz`); `LANE_CRATES`,
  `LANE_RELEASE_PLZ` and `ENABLE_CRATES_IO_PUBLISH` = `true`
- **touches (per crate):** `publish = true` in its `Cargo.toml`; its
  `release-plz.toml` block flips from `release = false` to
  `changelog_path = "CHANGELOG.md"`; manifest-audit allow-list entry.
- **verify:** `cargo publish --dry-run -p <crate>` then `just go`.

## lane:winget, Windows package manager

- **what:** `winget-publish.yml` PRs the manifest bump to microsoft/winget-pkgs
  on each release; `winget-token-expiry-check.yml` warns before the PAT dies.
- **prerequisites:** lane:release live; a first manual winget-pkgs submission
  establishing the package id.
- **secrets/vars:** `WINGET_TOKEN` (classic PAT); `LANE_WINGET=true`
- **verify:** dispatch `winget-publish.yml` manually against the latest release.

## lane:codecov, coverage reporting

- **what:** coverage upload from tier-2's coverage job; thresholds and PR
  comment layout in `codecov.yml` (project 90% / patch 95%).
- **secrets/vars:** `CODECOV_TOKEN`; `LANE_CODECOV=true`
- **verify:** next tier-2 run shows the upload; codecov dashboard populates.

## lane:codeql, code scanning uploads

- **what:** `codeql.yml` (weekly + PR SAST). The analysis is free everywhere,
  but *uploading results* requires a public repo or GitHub Advanced Security,
  so the job auto-enables on public repos and stays dormant on private ones.
- **secrets/vars:** nothing for public repos; `LANE_CODEQL=true` to force-on
  after purchasing GHAS for a private repo.
- **verify:** the next push shows "Analyze (rust)" running; findings appear
  under Security → Code scanning.

## lane:slsa, build provenance attestation

- **what:** `actions/attest-build-provenance` on release artifacts (Sigstore
  OIDC, no secrets).
- **prerequisites:** lane:release
- **secrets/vars:** `LANE_SLSA=true`
- **verify:** `gh attestation verify <artifact> --repo <owner>/<repo>`.

## lane:cross-lint, Windows/Linux/Intel-macOS cross-target lint gates

- **what:** `lint-ci-windows` (cargo-xwin clippy), `lint-ci-linux-zig`
  (cargo-zigbuild), `lint-ci-mac-intel` (x86_64-apple-darwin from an arm64
  host), `check-all-targets`. Recipes ship in `just/test.just` +
  `just/dev.just`; the three gates are in `scripts/ci/gates.toml` at
  pre-push and are required like every gate: a missing tool or target
  fails the push with the install command (`just install-dev-tools`).
- **tooling:** `cargo install cargo-xwin cargo-zigbuild`; `zig` at the
  version `scripts/ci/gates.toml` pins under `[toolchain]` (`just
  install-dev-tools` covers all three); `rustup target add
  x86_64-apple-darwin` (rust-toolchain.toml lists it).
- **touches:** nothing to enable locally (install the tools). The gates are
  already required; there is no `hard` field to flip and no way to make
  one optional.
- **verify:** `just check-all-targets`, then `just gates-drift`.

## lane:preview, preview binaries for a PR

- **what:** `preview-artifacts.yml` builds real binaries (Windows via
  cargo-xwin on Linux plus a Windows nextest-archive smoke, static-musl
  Linux, optionally macOS arm64 + Intel) for one SHA and uploads them with a
  SHA256 manifest. Never blocks a merge; refuses to build a SHA whose
  `PR Fast CI / required` is not green.
- **prerequisites:** none
- **secrets/vars:** none; the labels `preview-artifacts` and `preview-macos`
  (created by `bootstrap-github.sh`) or a `workflow_dispatch`.
- **runbook:** label the PR `preview-artifacts` (+ `preview-macos`), or
  `gh workflow run preview-artifacts.yml -f sha=<sha> -f targets=windows,linux,macos`.
- **verify:** the run's `manifest-<sha>` artifact lists every file with its
  hash, rustc and the tested SHA.

## lane:brew, Homebrew tap formula per release

- **what:** `brew-publish.yml` renders `packaging/homebrew/md2html.rb` with the
  release's asset URLs and SHA256s and pushes it to the tap repository;
  `release.yml` dispatches it after the release exists (same shape as winget).
- **prerequisites:** lane:release live; a tap repository
  (`<owner>/homebrew-md2html` by default, or the `HOMEBREW_TAP` variable).
- **secrets/vars:** `LANE_BREW=true`; `HOMEBREW_TAP_TOKEN` (fine-grained PAT,
  contents: write on the tap); optional `RELEASE_REPO` when releases live in a
  separate public repository.
- **verify:** `brew install <owner>/md2html/md2html` after the first release.

## lane:codesign, stable macOS binary identity and notarized releases

- **what:** `just/codesign.just` (`setup-codesign`, `build-signed`,
  `sign-binaries`, `doctor-codesign`, `resign-installed`,
  `secrets-sync-apple`), `packaging/macos/md2html.entitlements`, and the
  secrets-gated Developer ID + notarization steps in `release.yml`.
  Full story: `docs/forge/CODE-SIGNING.md`.
- **prerequisites:** none for development (a self-signed identity); an Apple
  Developer Program membership for notarized releases.
- **secrets/vars:** none locally (`MD2HTML_CODESIGN_IDENTITY` to pick an
  identity); for releases the six `APPLE_*` secrets `release.yml` reads,
  listed in `docs/forge/CODE-SIGNING.md` section 5. Absent secrets =
  ad-hoc signing.
- **runbook:** `just setup-codesign`, then `just build-signed` instead of
  `cargo build` when a macOS binary must keep one identity across builds.
- **verify:** `just doctor-codesign` reports every built binary as
  certificate-pinned.

## lane:main-guard, server-side backstop for unverified merges

- **what:** `main-merge-guard.yml` verifies every push to `main` came from a
  PR whose head carries a successful `PR Fast CI / required` aggregate and
  title check, auto-reverts one that did not (and files an issue); the
  pre-push hook refuses direct, deleting and force pushes to `main`; and
  `just merge` watches every check before merging. Together they make the
  rules hold on a plan without rulesets.
- **prerequisites:** none; always on. Once `just protect-main` has applied
  `scripts/ci/main-ruleset.json` (rulesets need a public repo or GitHub
  Pro/Team), the guard only ever confirms and can be retired.
- **verify:** `just merge` on a PR with a red check refuses; a direct
  `git push origin main` is refused by the hook.

---

## component:new-crate, add a workspace crate

- **what:** a new `crates/<name>` member following the canonical shape.
- **recipe:**
  1. Copy `scripts/dev-tools` as the skeleton (a lib with binaries; its one
     dependency is `anyhow`); rename the directory + `name =`. A crate is
     tooling: md2html itself takes none.
  2. Add it to `[workspace] members` and (if depended upon) as a dual
     path+version entry in `[workspace.dependencies]`.
  3. Add a `[[package]] name = "<name>" release = false` block to
     `release-plz.toml`.
  4. Keep every metadata field `*.workspace = true` and `[lints] workspace = true`;
     `md2html-manifest-audit` enforces it.
- **verify:** `cargo run -p md2html-manifest-audit && just go`

## component:fuzz-target, cargo-fuzz harness

- **what:** `fuzz/` dir under the crate that owns a parser/deserializer, plus a
  tier-2 fuzz job (a 10-minute bounded `cargo fuzz run <target> --
  -max_total_time=600`).
- **tooling:** `cargo install cargo-fuzz` (needs nightly, already pinned)
- **prerequisites:** an attack-surface API worth fuzzing (bytes → parse) in
  a tooling crate. Not md2html: a fuzz crate would be its first dependency;
  the generated documents and the differential corpus stand in for it
  (`just generated-documents`, `just differential`, `docs/TESTING.md`).
- **verify:** `cargo fuzz run <target> -- -runs=1000` locally; tier-2 green.

## component:bench-harness, criterion benchmarks

- **what:** `benches/` in a measured tooling crate; `criterion` via
  `[workspace.dependencies]`; a recipe that runs them. Not md2html:
  criterion would be its first dependency; `just measure` times its
  renders instead.
- **touches:** crate `Cargo.toml` (`[[bench]] harness = false`), workspace deps.
- **verify:** `cargo bench -p <crate> -- --test` (compile+smoke), `just go`.

## component:xtask-or-tool, internal tool crate

- **what:** a new tool under `scripts/` (pattern: `scripts/ci/md2html-gen-hooks`):
  `publish = false`, `release = false` block, full lint inheritance, clap.
- **verify:** `cargo run -p <tool> -- --help && just go`

---

The runbooks above are followed by hand and verified by the drift gates and
`just go`; no recipe runs them.
