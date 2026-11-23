# Supply chain: what md2html depends on, and on what basis

md2html is built from the Rust standard library alone. The binary has no third-party code in it; what it depends on is the toolchain that builds it and the tools that test it. Each is pinned to an exact version and was vetted as recorded here, on 2026-11-22; the tooling's crates as of 2026-11-23.

## How to read this

- **Chapter 1** is the whole list, one line each.
- **Chapters 2 to 4** are the evidence for each kind: the toolchain, the browser test tools, the CI actions.
- **Chapter 5** is how a change is vetted from now on.

## 1. The list

| Dependency | Version | Where | Ships in the binary |
|:--|:--|:--|:--:|
| Rust toolchain (rustc, cargo, rustfmt, clippy) | nightly-2026-11-23, chosen by `just toolchain-sync` | `rust-toolchain.toml` | no (compiler) |
| Oldest Rust promised | 1.85 | `rust-version`, CI job `msrv` | no |
| md2html's crates | none | `cargo tree -p md2html` | no |
| The build tooling's crates | 28, all fully audited (chapter 2) | `Cargo.lock`, `supply-chain/` | no |
| playwright-core | 1.65.1 | `tests/browser` (lockfile, `npm ci`) | no |
| axe-core | 4.14.1 | `tests/browser` | no |
| actions/checkout | v7.0.1, commit `3d3c42e5` | `.github/workflows/ci.yml` | no |
| actions/setup-node | v7.1.0, commit `949feb24` | `.github/workflows/ci.yml` | no |
| Node.js in CI | 26, the current LTS (Lithium) | `ci.yml` | no |
| cargo-deny, cargo-vet (CI tools) | 0.22.0, 0.10.3 | `ci.yml`, installed `--locked` | no |

Not dependencies of the repository, and not vetted here: `mermaid.js`, which an author may hand to `--mermaid-js` (it then goes into that author's page, so its provenance is that author's to check).

## 2. The toolchain and the crates

- **Toolchain.** `rust-toolchain.toml` pins a dated nightly (2026-11-23; a nightly because the formatting rules use rustfmt options that are not yet stable), with rustfmt, clippy and the cross-lint targets; CI installs from that file, so local and CI builds use the same compiler. `just toolchain-sync` chooses the pin: the newest nightly on which clippy passes for both feature sets, for Windows (cargo-xwin) and for Linux (cargo-zigbuild); it steps back a day at a time until one does. md2html itself needs nothing unstable: a separate CI job builds it with Rust 1.85, the oldest it promises.
- **md2html's crates: none.** `cargo tree -p md2html` shows md2html alone; the binary is the Rust standard library and md2html.
- **The build tooling's crates.** The repository is a rust-forge workspace: beside md2html it holds the forge's own tools (the CI pipeline, the generators of the git hooks and workflows, the manifest audit), which use 28 third-party crates: clap for their command lines, serde with serde_json and toml for the pipeline's state file and the manifests they read, regex for the workflow validator's line patterns, anyhow for error chains, and what those bring. They build and check md2html; none of them is linked into it. The pipeline runs `cargo` and `git` as child processes and needs nothing else for it: its parallel steps, time limits, colour and run ids are the standard library's (until 2026-11-23 it used tokio, futures, indicatif, uuid, num_cpus and colored, which brought the count to 90). `cargo deny check` (licences, advisories, bans, sources) and `cargo vet --locked` run on every change, locally in the pre-push gate and in CI.
- **The audit store** (`supply-chain/`) stands on its own: audits imported from Mozilla, Google, the Bytecode Alliance, ISRG and Zcash, and this repository's own. All 28 crates are fully audited, by an imported audit, a publisher the store trusts, or an audit of this repository's own; none is exempted (`supply-chain/config.toml` has no `[[exemptions]]`), and `deny.toml` allows only the four licences these crates use (MIT, Apache-2.0, Unlicense, Unicode-3.0). It refuses two versions of one crate (`Cargo.lock` has none), a wildcard version, and any registry or git source other than crates.io. `deny.toml` ignores no advisory. (It ignored one until 2026-11-22: `chrono`, flagged unmaintained and used by the pipeline tool for three timestamps. The tool now reads the clock itself, which also removed 16 crates and half the exemptions.)
- **A crate update is read, or it waits.** An update from a publisher the store trusts may be taken without an audit once it is a week old. Younger than that, its diff is read and the audit recorded first: of the sixteen crate versions taken on 2026-11-22, thirteen were younger than a week or had no trusted publisher, and each of those has its delta audit in `supply-chain/audits.toml`; the other three (bumpalo, clap_lex, mio) were a week old or more, from publishers the store trusts.

## 3. The browser test tools

Both run only in tests, from a committed lockfile, with install scripts disabled (`npm ci --ignore-scripts`).

| Check | playwright-core 1.65.1 | axe-core 4.14.1 |
|:--|:--|:--|
| Published | 2026-11-13 | 2026-11-16 |
| Publisher | Microsoft (microsoft-oss-releases, maintainers of microsoft/playwright) | Deque (axe, dequelabs/axe-core) |
| Licence | Apache-2.0 | MPL-2.0 (a test tool, not distributed with md2html) |
| Dependencies | none | none |
| Install scripts | none | none |
| Registry signatures | verified | verified |
| Provenance (SLSA, built by the project's own GitHub workflow) | verified | verified |
| Known vulnerabilities (`npm audit`) | none | none |

Both are newer than the week's cooldown for automatic proposals (chapter 5). They were taken anyway because the provenance attestation already proves what a cooldown waits to find out: that the published package was built from the project's own repository by its own workflow. The audit and the click-through were run on them before they were committed. The new axe-core found a real issue the old one missed (the search button's name did not include its visible text, WCAG 2.5.3), which was fixed.

## 4. The CI actions

Pinned by commit, not by tag, because a tag can be moved and a commit cannot.

| Action | Release | Commit | Signature | On the main history | Runtime |
|:--|:--|:--|:--|:--|:--|
| actions/checkout | v7.0.1 (2026-07-17) | `3d3c42e5aac5ba805825da76410c181273ba90b1` | GitHub-verified | yes | node24 |
| actions/setup-node | v7.1.0 (2026-10-08) | `949feb2413d6458794dcd2491c4babbbce0c15c1` | GitHub-verified | yes | node24 |

The one breaking change in their recent releases concerns `pull_request_target`, which md2html does not use. Checkout runs with `persist-credentials: false`: no job pushes, so no job keeps a token on disk. The workflow's token is read-only.

## 5. Vetting a change

Dependabot proposes updates to the actions, the browser tools and the tooling's crates, weekly, and only for releases at least seven days old (`cooldown` in `.github/dependabot.yml`, for all three). A proposal is merged when:

1. the release comes from the same publisher, signed, with provenance where the ecosystem has it (npm, GitHub);
2. it brings no new dependency and no install script, or each one it brings is vetted the same way;
3. `npm audit`, `cargo deny check` and `cargo vet` pass;
4. the whole CI passes on it, the accessibility audit and the click-through included;
5. this page is updated with the new version and its evidence.

A first crate is a decision of its own: md2html's promise is that it has none, so a crate comes in only with a reason written here, its `cargo vet` audit or family trust, and the size of what it brings.
