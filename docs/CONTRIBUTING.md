<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Contributing to md2html

Thank you for looking. md2html turns Markdown into one self-contained HTML page, and it is written against the Rust standard library alone. This page is for a person who wants to change md2html: how to build it, what a change has to keep true, how a change reaches `main`, and which document to read next.

- **A bug or an idea:** open an issue.
- **A security problem:** report it privately, as [SECURITY.md](../SECURITY.md) describes.

## Build and try it

You need Rust (the pinned toolchain installs itself from `rust-toolchain.toml`; `just setup` adds the components and targets the gates use) and [`just`](https://just.systems).

```sh
cargo build --release            # target/release/md2html
cargo test                       # the tests and the CommonMark examples
just use-local                   # build and install to ~/bin
just release-local               # archives for six platforms, in dist/
```

`just setup` installs the toolchain and every gate tool (the cross-build tools too, through `just install-dev-tools`) and wires the git hooks. [docs/forge/GETTING-STARTED.md](forge/GETTING-STARTED.md) walks a new machine through it, signing included.

## What to read, in order

1. This page: the rules a change keeps and how it lands.
2. [docs/forge/GETTING-STARTED.md](forge/GETTING-STARTED.md): the machine, the hooks, the daily loop and the fix for each gate.
3. [ARCHITECTURE.md](ARCHITECTURE.md): md2html's modules in layers, and which way each may use another.
4. The policies in [docs/policies/](policies/), [lint-posture.md](policies/lint-posture.md) first: why each rule is what it is, and what to write instead of what a lint refuses.
5. [TESTING.md](TESTING.md): each kind of test, what it guards, where it runs and how to add one.

[GOTO.md](../GOTO.md) says where the project stands; [AGENTS.md](../AGENTS.md) holds the same rules as a literal checklist; [MAINTAINING.md](MAINTAINING.md) says how to take the project over and lists every recurring chore (the toolchain, the pinned tools and their checksums, epubcheck, the runners), with how to do each.

## The toolchain

- **Development, CI and every gate** use the dated nightly pinned in `rust-toolchain.toml`, because the formatting rules use rustfmt options that are not yet stable. `just toolchain-sync` chooses the pin.
- **md2html needs nothing unstable.** It builds with the Rust its `rust-version` names, in `crates/md2html/Cargo.toml`, the one place that number is written for tools (the README, `docs/SUPPLY-CHAIN.md` and the release policy state it to readers, and the gate `versions` holds them to it); the `msrv` gate and CI's `msrv` job build it and run its tests with exactly that Rust, so md2html's code uses nothing newer and behaves the same there.
- **The tooling crates declare no `rust-version`.** They are built only with the pinned toolchain. The root `Cargo.toml` says why, and `md2html-manifest-audit` (gate `manifest-drift`) keeps it so.

## What a change keeps true

1. **No dependencies.** `crates/md2html` uses the standard library only. A change that needs a crate is a conversation first.
2. **CommonMark may rise, never fall.** The number of passing specification examples is a floor.
3. **No script from the text, ever.** [SECURITY.md](../SECURITY.md) lists the guarantees. A change near `sanitize.rs`, `text.rs` or the embedding of files comes with a test that shows the guarantee still holds.
4. **Pages are compared.** A change not meant to alter output leaves the rendered README, showcase and capabilities page byte for byte the same. A change meant to alter it runs the browser suite in `tests/browser` (`node audit.js`, `node interact.js`, `node live.js`, `node nesting.js`).
5. **`--check` finds nothing.** `md2html --check README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md`.
6. **Every item is documented**, private ones too, and all code meets the whole lint table in the root `Cargo.toml`: no `#[allow]`, no `#[expect]`, but the one exception the owner approved (`clippy::float_arithmetic` in `crates/md2html/src/mermaid/geom.rs`, [lint-posture.md](policies/lint-posture.md)). A finding is fixed at its cause, usually the type or structure it points at (see the top of `AGENTS.md`).
7. **The command line is a contract.** [command-line.md](command-line.md) lists every option, setting, exit code and environment variable, and a test fails when the page and the code disagree. A change to any of them changes the page in the same pull request and is named in the changelog.
8. **Uses go down the layers.** [ARCHITECTURE.md](ARCHITECTURE.md) places each of md2html's modules in a layer: a module uses its own layer or a lower one, and no uses close a circle. A test holds it.

## How a change reaches main

Nothing is pushed to `main`. A change is a branch, a pull request, and a merge commit.

```sh
git switch -c fix/short-description
# edit
just check                       # seconds: compile, format, file size
just lint-ci                     # clippy, as CI runs it
just go                          # the definition of done
git commit -m "fix: what changed"
git push -u origin fix/short-description
just pr "fix: what changed"      # opens the pull request, title checked first
just merge <number>              # waits for every check, then merges
```

- **Commits are signed.** `just setup-signing` configures it for a clone; `just doctor-signing` checks it.
- **The hooks are the gates.** A commit or a push that is refused names the gate that refused it; [AGENTS.md](../AGENTS.md), section 6, and [docs/forge/GETTING-STARTED.md](forge/GETTING-STARTED.md) have the fix for each. A gate is never bypassed (`--no-verify`); if one is wrong, say so in the pull request.
- **Merges are merge commits**, so every signed commit stays in the history. Squash and rebase merges are switched off.

### Commit message conventions

Commits and pull request titles follow [Conventional Commits](https://www.conventionalcommits.org): `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `build`, `perf`, `style`, `revert`, with at most one scope (`feat(build): ...`) and `!` for a breaking change. The subject says what changed, in the present tense. The gate `commit-subjects` checks every pushed commit, and `commitlint.yml` the pull request's title, with the same pattern (`scripts/ci/check_commit_subjects.sh`).

## Where things are

| Path | What it is |
| --- | --- |
| `crates/md2html/src` | md2html: one binary. `page/` holds the page's own CSS and JavaScript |
| `docs/ARCHITECTURE.md` | md2html's modules in layers, and which way each may use another |
| `docs/command-line.md` | What the command line promises: options, settings, exit codes, environment |
| `crates/md2html/tests/commonmark` | The specification's examples |
| `crates/md2html/tests/command_line` | Tests that run the built program: exit codes, streams, `--watch`, `--pdf` |
| `docs/TESTING.md` | Each kind of test, what it guards and where it runs |
| `docs/policies/` | Why each rule is what it is: lints, panics, allocation, types, concurrency, platforms, hostile input, dependencies, build scripts |
| `SECURITY.md`, `docs/SUPPLY-CHAIN.md` | What md2html promises about safety, and the evidence for every crate and action the build uses |
| `tests/browser` | The rendered pages in Chrome: accessibility, click-through, live reload |
| `docs/capabilities.md` | Everything md2html does, and its sixteen translations beside it |
| `scripts/dev-tools` | `just use-local` and `just release-local` |
| `scripts/ci`, `just/`, `.github/workflows` | The gates, the recipes and CI |
| `GOTO.md` | Where the project stands and what is switched off |
| `docs/MAINTAINING.md` | Taking the project over, deprecations, and every recurring chore with how to do it |
| `AGENTS.md` | The same rules, as a literal checklist |

## Translations

A translation is the Markdown file beside its original, named for its language (`capabilities.de.md`). It records its original and the original's fingerprint (`translated_from`, `source_sha256`). Editing `docs/capabilities.md` makes every translation stale until it is brought up to date and its fingerprint renewed; `--check` says which. None of the sixteen, nor md2html's interface words in them (`crates/md2html/src/config/words/`), was written by a native speaker: a native speaker's corrections are very welcome, and [issue #65](https://github.com/githubrobbi/md2html/issues/65) says what to look at first and how to send them.

## Licence

md2html is under MIT or Apache-2.0, at your choice. A contribution is offered under the same two licences.
