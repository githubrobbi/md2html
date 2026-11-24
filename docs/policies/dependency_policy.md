<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Dependencies and features

For whoever would add a crate or a feature: why md2html has no dependencies, and the rules the tooling's crates and features follow.

## md2html has none

`crates/md2html` is the Rust standard library and its own code: no `[dependencies]`, no build script, no features. The README promises it, and it is what keeps the binary small, the build reproducible and the supply chain short. A first crate is a decision for the owner, taken with its reason, its audit and its size written into [docs/SUPPLY-CHAIN.md](../SUPPLY-CHAIN.md) (chapter 5); [AGENTS.md](../../AGENTS.md), section 7, says to stop and ask.

Enforced by: review, and gates `vet` and `deny`, which refuse a crate that has no audit or a licence outside `deny.toml`. CI runs `cargo deny check` and `cargo vet --locked` on every pull request (job `supply-chain` in `ci.yml`).

## The tooling's crates

The crates under `scripts/` and `crates/md2html-version` build and check md2html; none of them ships in it. They use what the standard library does not have, and nothing more: `clap` for the command lines (default features off), `serde` with `serde_json` and `toml` for the pipeline's state file and the manifests, `regex` for the workflow validator's line patterns (default features off: the standard library and the Unicode Perl classes the patterns use), and `anyhow` for error chains. With what those bring, that is 28 third-party crates in `Cargo.lock`, each audited; 27 of them are built (Cargo locks an optional dependency whatever the features, and no feature the tooling asks for builds `aho-corasick`). Parallel steps, time limits, colour and the clock are the standard library's ([concurrency_policy.md](concurrency_policy.md)); on 2026-11-23 the pipeline left tokio, futures, indicatif, uuid, num_cpus and colored behind, and the count went from 90 to 28. [docs/SUPPLY-CHAIN.md](../SUPPLY-CHAIN.md) holds the evidence for each crate.

| Rule | Enforced by |
| --- | --- |
| A version is written once, in the root `Cargo.toml` under `[workspace.dependencies]`; a crate says `name.workspace = true`. | `md2html-manifest-audit`, gate `manifest-drift` |
| Every build, test and check resolves through the committed `Cargo.lock` (`--locked`), so what is tested, released and audited is the same set of versions. | The cargo commands of the gates (`scripts/ci/gates.toml`) and of the workflows under `.github/workflows/` pass `--locked` (cargo-mutants through `.cargo/mutants.toml`); review |
| Every dependency a crate declares is used. | Gate `machete` on every push; the weekly `udeps` job, which compiles to be sure |
| Licences, advisories and sources are within `deny.toml`: four licences are allowed (MIT, Apache-2.0, Unlicense, Unicode-3.0), no advisory is ignored, and two versions of one crate, a wildcard version and a registry or git source other than crates.io are refused. | Gate `deny` |
| Every crate version is fully audited or comes from a publisher the audit store trusts, in `supply-chain/`, changed only with `cargo vet` commands ([AGENTS.md](../../AGENTS.md), prohibition 6). There is no exemption (`supply-chain/config.toml` has no `[[exemptions]]`), and none is added to make the gate pass. | Gates `vet`, `vet-audit-discipline` |

How an update is vetted, by whom and on what evidence, is in [docs/SUPPLY-CHAIN.md](../SUPPLY-CHAIN.md), chapter 5. For Dependabot's patch updates, `.github/workflows/dependabot-auto-merge.yml` turns on GitHub's auto-merge, which waits for every required check, `vet` and `deny` among them; minor and major updates are reviewed by a person.

## Features

The one feature in the workspace is `build` in `md2html-version`: it turns on `emit_build_env`, which the tooling's build scripts call, and only their `[build-dependencies]` enable it, so no binary compiles it in.

A feature adds and never takes away. Cargo builds a crate once with the union of the features every user asks for, so a feature that removes an item would remove it from users that never asked.

Enforced by: gate `lint-ci-no-default` (Clippy with no default features) beside gate `lint-ci` (all features), so code that compiles only with one feature on, or only with it off, fails one of them; the weekly `hack` job builds each feature on its own.
