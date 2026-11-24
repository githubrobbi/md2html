<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Getting Started: the zero-knowledge runbook

This guide is for a person who wants to work on md2html and assumes
**nothing**: not Rust, not `just`, not GitHub CLI. Follow it top to bottom
and you end up with md2html building on your machine inside its quality
machine (strict lints, generated git hooks, tiered CI, supply-chain
checks). The rules a change keeps are in
[docs/CONTRIBUTING.md](../CONTRIBUTING.md); come back to it after Step 4.

## Completely new to git or GitHub?

You need three concepts before anything here makes sense. Don't learn them
from us; these are the canonical guides, and they are excellent:

| Concept | What it is for | Learn it here (15-30 min each) |
| --- | --- | --- |
| **git** | The version-control tool: every change to your code is recorded as a "commit" you can inspect, undo, and share. Everything in this repository hangs off git. | [git-scm.com/book, chapters 1-2](https://git-scm.com/book/en/v2/Getting-Started-About-Version-Control) |
| **GitHub** | The hosting service where your repository lives online: backups, collaboration, pull requests, and the CI that re-checks your work. You need a (free) account. | [Create an account](https://github.com/signup), then [GitHub's Hello World](https://docs.github.com/en/get-started/start-your-journey/hello-world) |
| **SSH keys** | A key *pair*: the private half stays on your machine and does the signing/authenticating; the public half gets registered with GitHub so it can verify you. That is why setup always has two sides: local (create + configure) and GitHub (register). One key can serve both *authentication* (proving it's you when pushing) and *commit signing* (proving each commit came from you), but GitHub registers those as two separate entries. | [GitHub: About SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/about-ssh) · [About commit signature verification](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification) |

Come back when those click; the rest of this guide automates nearly all of
the mechanics for you.

**The one rule of this repository:** the gates are the law. When a commit
or push is rejected, the machine is telling you exactly what to fix. Fix
the cause; never bypass (`--no-verify` is treated as an incident).

---

## Step 0: Install the base tools (once per machine)

Compiler prerequisites, package manager, and the CLI tools, by hand:

macOS: `xcode-select --install`, then [Homebrew](https://brew.sh), then
`brew install just gh jq pipx && pipx install reuse`.
Linux (Debian/Ubuntu): `sudo apt install build-essential curl git jq pipx`,
`pipx install reuse`, plus [`just`](https://github.com/casey/just#packages)
and [`gh`](https://cli.github.com). Both: install Rust via
[rustup](https://rustup.rs), then authenticate with `gh auth login`.

You do **not** need to pick a Rust version; the repo pins its own toolchain
in `rust-toolchain.toml`, and rustup installs it automatically on first build.

## Step 0.5: Set up commit signing (required, one command)

The pre-push gate verifies that **every commit is signed**, a hard gate, so
do this before your first push, not after it rejects you. Everything that
can be automated, is:

```bash
just setup-signing
```

That single command: generates an SSH key if you have none (you choose the
passphrase), configures **this repository** to sign every commit and tag
with it (repo-local git config; your global config and every other
repository on the machine stay untouched), registers
the public half on GitHub as a signing key via the API, and finishes with
`just doctor-signing` to prove the whole chain. Idempotent: re-run it any
time; it never overwrites an existing key.

The only case needing a manual step: if `gh` says its token lacks the
signing-key scope, run `gh auth refresh -h github.com -s admin:ssh_signing_key`
and re-run the recipe.

Classic GPG works too (configure `user.signingkey <KEYID>` yourself);
`just doctor-signing` diagnoses either setup, and `just sign-branch` can
rescue a branch that accumulated unsigned commits.

---

## Step 1: Get the code onto your machine

```bash
gh repo clone githubrobbi/md2html     # or: git clone git@github.com:githubrobbi/md2html.git
cd md2html
```

To propose changes without write access, fork it first (`gh repo fork
githubrobbi/md2html --clone`). Step 2 below records what is already done.

## Step 2: The project identity (already done)

md2html adopted the forge's scaffolding (`docs/forge/FORGE-STAMP.toml`
names the template version), so the identity is in place: names,
manifests, workflows and licences already say md2html, and the project is
dual-licensed **MIT OR Apache-2.0**. There is no identity ceremony to run
here; a new project starts from the template itself, not from this
repository. Later template changes arrive with `just forge-upgrade`.

## Step 3: Set up your environment (once per machine)

```bash
just setup
```

This installs the pinned toolchain, every tool the gates call
(`cargo-nextest`, `cargo-deny`, `cargo-vet`, `cargo-machete`, `typos`,
`taplo`, ...), wires the git hooks, and smoke-checks the workspace.
Idempotent: re-run it any time.

Build caching is **resilient**: every `just`-driven build (recipes, hooks,
the go/ship pipeline) auto-detects an installed, functional `sccache` and
uses it, and silently runs stock Cargo when it is absent. `just setup`
installs sccache, so you get warm-cache speed from day one without the repo
ever *requiring* it. Plain `cargo` invocations stay stock unless you opt in
user-level: see "Performance tuning (optional)" at the end of this guide.

## Step 4: Prove the machine

```bash
just go
```

Green means: toolchain pinned, file sizes in policy, all tests pass, doc-tests
pass, production and test lints clean, dependencies vetted and license-clean,
docs build without warnings. This is your "everything is fine" button -
run it whenever you want certainty.

## Step 5: The GitHub-side state (already done)

Rulesets, labels and lane switches live on GitHub, not in the files. For
md2html they are in place: `scripts/ci/bootstrap-github.sh` set them up,
and the `main-protection` ruleset (no deletion, no force push, signed
commits, a pull request with every thread resolved, the required checks;
the data is `scripts/ci/main-ruleset.json`, re-applied with `just
protect-main`) is active. There is no merge queue: GitHub offers one only
to repositories an organization owns (`GOTO.md`). Beside the ruleset, the
pre-push hook refuses direct pushes to `main`, `just merge` waits for
every check, and `main-merge-guard.yml` reverts a merge that slipped
through unverified.

---

## Daily life (the vibe-coding loop)

Your project's **living state doc is `GOTO.md`**: where you are in the
flow, what the next weeks look like, and which dormant lane does what.
Start there when returning after a break, and keep it updated as state
changes.

```bash
# 1. A branch of your own; never work on main
git switch -c feat/my-change
# 2. Make changes, with quick feedback while working:
just check          # compile, format, file size: seconds
just lint-ci        # clippy, as CI runs it
just test           # full test suite via nextest
# 3. Commit; the pre-commit hook runs automatically (~2-15 s)
git add -A && git commit -m "feat: describe the change"
# 4. Push; the pre-push hook runs the big battery (~20-60 s) and refuses a push to main
git push -u origin feat/my-change
# 5. Open a PR; CI runs the same gates, plus more
just pr "feat: describe the change"   # title validated locally first
# 6. Merge it: watches EVERY check, then merges (never raw `gh pr merge`)
just merge <PR-number>
```

`just pre-pr` rehearses the whole PR tier locally before step 4 when you
would rather spend minutes here than paid minutes on a red PR.

Commit messages and pull request titles follow Conventional Commits
(`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`,
`chore`, `style`, `revert`, at most one scope); the gate `commit-subjects`
refuses a pushed commit with any other subject
([CONTRIBUTING.md](../CONTRIBUTING.md#commit-message-conventions)).

### When a gate fails

The failing gate prints its name. Reproduce it locally, fix the cause,
retry. The fix for every gate is one table, in
[AGENTS.md, section 6](../../AGENTS.md#6-when-a-gate-fails-exact-fixes),
written for agents and people alike; the gate `doc-references` fails when
a gate of `scripts/ci/gates.toml` has no row there, so the table cannot
fall behind. The rule behind each lint, and what to write instead of what
it refuses, is in [docs/policies/lint-posture.md](../policies/lint-posture.md).

### The rules in checklist form

`AGENTS.md` states how this repo works as literal rules (commands, lint
rules, conventions, "never edit generated hooks", "never bypass gates").
However a change is made, the gates are what make it safe: it must pass
the same checks as every other change.

---

## Growing the project

- **Add structure** (a tooling crate, a component): follow the recipes in
  [`COMPONENTS.md`](COMPONENTS.md). Every recipe ends in `just go`.
  md2html itself takes no crate, so no benchmark or fuzzing crate either:
  `just measure` times a render, and `just differential` and `just
  generated-documents` stand in for fuzzing ([TESTING.md](../TESTING.md)).
- **Switch on dormant machinery** (GitHub releases, crates.io publishing,
  winget, codecov, SLSA): each is a `lane:*` entry in `COMPONENTS.md` -
  activation is a repo variable plus, at most, a secret. Nothing needs to
  be built. Each lane is the owner's decision.
- **Ship**: `just ship` validates, bumps the version, rolls the changelog,
  opens the release PR; `just merge <PR>` lands it as a merge commit. (Enable
  `LANE_RELEASE` first if you want merged release PRs to build binaries.)

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `cargo nextest: command not found` | `just setup` |
| First build downloads a whole toolchain | Expected: `rust-toolchain.toml` pins a specific nightly; rustup fetches it once |
| Commit succeeded without any hook output | Hooks not wired: `just install-hooks` |
| `reuse: command not found` (or another tool) in hook output | Every gate is required and its tool with it: `just install-dev-tools` |
| Push rejected: `vet-audit-discipline` | A crate's version changed without its audit: `just vet-bump CRATE NEW OLD`, or `cargo vet certify` with the evidence ([SUPPLY-CHAIN.md](../SUPPLY-CHAIN.md), chapter 5) |
| CI job "Analyze (rust)" skipped | CodeQL needs a public repo (or GHAS + `LANE_CODEQL=true`), by design |
| No merge queue on PRs | GitHub offers one only to repositories an organization owns; this one belongs to a user account (`GOTO.md`) |
| `just: command not found` on Windows | Use `winget install Casey.Just`; hooks run under Git-Bash |
| Managed/corporate device (MDM) blocks a tool install (LLVM/clang, zig, ...) | The gates need their tools: the cross-target lint gates (`lane:cross-lint`: cargo-xwin, cargo-zigbuild with zig) are required at pre-push like every other gate, so a machine that cannot install them cannot push from there. Check your employer's acceptable-use policy before requesting an install; personal projects belong on personal hardware |

Still stuck? `just` (no arguments) lists every available command with a
one-line description.

---

## Performance tuning (optional)

The repo deliberately ships **stock Cargo behavior**; speed-ups are personal,
machine-level choices in `~/.cargo/config.toml` (never committed):

Common choices: sccache (also for plain `cargo` and your IDE), a faster
linker (mold on Linux, rust-lld), a target folder outside a synced folder
(Dropbox, OneDrive), Defender exclusions on Windows. Never
`target-cpu=native` for anything you hand to someone else.

Helpers: `just setup-sccache` (install + cache stats), `just sccache-stats`.
Note that `just`-driven builds already use sccache automatically whenever it
is installed; the user-level config extends that to plain `cargo` and your
IDE's builds.

A warning from experience: redirecting `target-dir` to a shared location
(e.g. `/tmp/rust-target`) makes `cargo clean` and repo deletion stop
reclaiming space: the shared cache silently grows by hundreds of GB across
workspaces until the volume fills and everything on the machine starts
failing with "no space left on device". If you must redirect, put it on a
volume you monitor.
