# Supply chain: what md2html depends on, and on what basis

md2html is built from the Rust standard library alone. The binary has no third-party code in it; what it depends on is the toolchain that builds it and the tools that test it. Each is pinned to an exact version and was vetted as recorded here, on 2026-11-22.

## How to read this

- **Chapter 1** is the whole list, one line each.
- **Chapters 2 to 4** are the evidence for each kind: the toolchain, the browser test tools, the CI actions.
- **Chapter 5** is how a change is vetted from now on.

## 1. The list

| Dependency | Version | Where | Ships in the binary |
|:--|:--|:--|:--:|
| Rust toolchain (rustc, cargo, rustfmt, clippy) | 1.100.0, the latest stable | `rust-toolchain.toml` | no (compiler) |
| Oldest Rust promised | 1.85 | `rust-version`, CI job `msrv` | no |
| Crates | none | `Cargo.lock` holds md2html alone | no |
| playwright-core | 1.65.1 | `tests/browser` (lockfile, `npm ci`) | no |
| axe-core | 4.14.1 | `tests/browser` | no |
| actions/checkout | v7.0.1, commit `3d3c42e5` | `.github/workflows/ci.yml` | no |
| actions/setup-node | v7.1.0, commit `949feb24` | `.github/workflows/ci.yml` | no |
| Node.js in CI | 26, the current LTS (Lithium) | `ci.yml` | no |
| cargo-deny, cargo-vet (CI tools) | 0.22.0, 0.10.3 | `ci.yml`, installed `--locked` | no |

Not dependencies of the repository, and not vetted here: `mermaid.js`, which an author may hand to `--mermaid-js` (it then goes into that author's page, so its provenance is that author's to check), and the deploy tool of the 4sons.com page (`wrangler`, pinned at 4.154.0 like the family's other page builds; 4.166.0 was two days old and waits out the cooldown below).

## 2. The toolchain and the crates

- **Toolchain.** `rust-toolchain.toml` pins 1.100.0 (2026-11-09) with rustfmt and clippy; CI installs from that file, so local and CI builds use the same compiler. Clippy runs with the pedantic lints as errors.
- **Crates: none.** `cargo deny check` (licences, advisories, bans, sources) and `cargo vet --locked` both run in CI and both pass on the empty graph: "Vetting Succeeded (because you have no third-party dependencies)". `deny.toml` is the policy a first crate would meet: permissive licences only, no unknown registry or git source, no wildcard versions, no duplicate versions, no RustSec advisory. `supply-chain/` holds no exemptions; a crate added later is audited or trusted with `cargo vet`, the way the family's other repositories do it (see iv's `docs/SUPPLY-CHAIN.md`), never exempted to make the gate pass.

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

Dependabot proposes updates to the actions, the browser tools and (should one ever exist) a crate, weekly, and only for releases at least seven days old. A proposal is merged when:

1. the release comes from the same publisher, signed, with provenance where the ecosystem has it (npm, GitHub);
2. it brings no new dependency and no install script, or each one it brings is vetted the same way;
3. `npm audit`, `cargo deny check` and `cargo vet` pass;
4. the whole CI passes on it, the accessibility audit and the click-through included;
5. this page is updated with the new version and its evidence.

A first crate is a decision of its own: md2html's promise is that it has none, so a crate comes in only with a reason written here, its `cargo vet` audit or family trust, and the size of what it brings.
