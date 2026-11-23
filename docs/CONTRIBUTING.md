<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Contributing to md2html

Thank you for looking. md2html turns Markdown into one self-contained HTML page, and it is written against the Rust standard library alone. This page says how to build it, what a change has to keep true, and how a change reaches `main`.

- **A bug or an idea:** open an issue.
- **A security problem:** report it privately, as [SECURITY.md](../SECURITY.md) describes.

## Build and try it

You need Rust (the pinned toolchain installs itself from `rust-toolchain.toml`) and [`just`](https://just.systems).

```sh
cargo build --release            # target/release/md2html
cargo test                       # the tests and the CommonMark examples
just use-local                   # build and install to ~/bin
just release-local               # archives for six platforms, in dist/
```

`just setup` installs the gate tools and wires the git hooks; `just install-dev-tools` adds the cross-build tools.

## The toolchain

- **Development, CI and every gate** use the dated nightly pinned in `rust-toolchain.toml`, because the formatting rules use rustfmt options that are not yet stable. `just toolchain-sync` chooses the pin.
- **md2html needs nothing unstable.** It builds with the Rust its `rust-version` names, in `crates/md2html/Cargo.toml`, the one place that number is written; the `msrv` gate and CI's `msrv` job build it with exactly that Rust, so md2html's code uses nothing newer.
- **The tooling crates declare no `rust-version`.** They are built only with the pinned toolchain. The root `Cargo.toml` says why, and `md2html-manifest-audit` (gate `manifest-drift`) keeps it so.

## What a change keeps true

1. **No dependencies.** `crates/md2html` uses the standard library only. A change that needs a crate is a conversation first.
2. **CommonMark may rise, never fall.** The number of passing specification examples is a floor.
3. **No script from the text, ever.** [SECURITY.md](../SECURITY.md) lists the guarantees. A change near `sanitize.rs`, `text.rs` or the embedding of files comes with a test that shows the guarantee still holds.
4. **Pages are compared.** A change not meant to alter output leaves the rendered README, showcase and capabilities page byte for byte the same. A change meant to alter it runs the browser suite in `tests/browser` (`node audit.js`, `node interact.js`, `node live.js`).
5. **`--check` finds nothing.** `md2html --check README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md`.
6. **Every item is documented**, private ones too, and all code meets the whole lint table in the root `Cargo.toml` with no exception: no `#[allow]`, no `#[expect]`. A finding is fixed at its cause, usually the type or structure it points at (see the top of `AGENTS.md`).

## How a change reaches main

Nothing is pushed to `main`. A change is a branch, a pull request, and a merge commit.

```sh
git switch -c fix/short-description
# edit
just check                       # seconds: compile and lint
just go                          # the definition of done
git commit -m "fix: what changed"
git push -u origin fix/short-description
just pr "fix: what changed"      # opens the pull request, title checked first
just merge <number>              # waits for every check, then merges
```

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org): `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `build`, `perf`, `style`, `revert`, with at most one scope (`feat(build): ...`). The subject says what changed, in the present tense.
- **Commits are signed.** `just setup-signing` configures it for a clone; `just doctor-signing` checks it.
- **The hooks are the gates.** A commit or a push that is refused names the gate that refused it; [docs/forge/GETTING-STARTED.md](forge/GETTING-STARTED.md) has the fix for each. A gate is never bypassed (`--no-verify`); if one is wrong, say so in the pull request.
- **Merges are merge commits**, so every signed commit stays in the history. Squash and rebase merges are switched off.

## Where things are

| Path | What it is |
| --- | --- |
| `crates/md2html/src` | md2html: one binary. `page/` holds the page's own CSS and JavaScript |
| `crates/md2html/tests/commonmark` | The specification's examples |
| `tests/browser` | The rendered pages in Chrome: accessibility, click-through, live reload |
| `docs/capabilities.md` | Everything md2html does, and its sixteen translations beside it |
| `scripts/dev-tools` | `just use-local` and `just release-local` |
| `scripts/ci`, `just/`, `.github/workflows` | The gates, the recipes and CI |
| `GOTO.md` | Where the project stands and what is switched off |
| `AGENTS.md` | The same rules, as a literal checklist |

## Translations

A translation is the Markdown file beside its original, named for its language (`capabilities.de.md`). It records its original and the original's fingerprint (`translated_from`, `source_sha256`). Editing `docs/capabilities.md` makes every translation stale until it is brought up to date and its fingerprint renewed; `--check` says which. A native speaker's corrections to any of the sixteen are very welcome.

## Licence

md2html is under MIT or Apache-2.0, at your choice. A contribution is offered under the same two licences.
