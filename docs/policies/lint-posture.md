<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# The lint posture

What the compiler, Clippy and rustdoc check in this workspace, where that is set, which gate runs it, and what to do when a lint fires. The other documents in this folder give the reasoning for particular groups of lints:

- [panic_policy.md](panic_policy.md): production code returns errors, it does not panic.
- [allocation_policy.md](allocation_policy.md): borrow before copying, write into one string.
- [type_policy.md](type_policy.md): data gets a type that says what it is.
- [concurrency_policy.md](concurrency_policy.md): the one place with threads.
- [hostile_input_policy.md](hostile_input_policy.md): what the compiler does not catch in a tool that reads other people's text.
- [dependency_policy.md](dependency_policy.md) and [build_codegen_policy.md](build_codegen_policy.md): what the workspace is built from.

## The rule

A lint finding is a defect and gets a real fix. There are no `#[allow(...)]` and no `#[expect(...)]` attributes in this code, in any form, tests and tooling included, apart from the one the owner decided (below), and none are added. Nothing stands in for one either: no exceptions list, no marker comment, no wrapper or macro whose purpose is that a lint cannot see through it.

A finding is fixed at its cause, and the cause is usually the data structure or the type, not the line where the lint fires. When the same small fix would be needed in many places, a type or a helper is missing. The table under "Fixing a finding" lists the causes this code has met.

If something truly cannot be fixed without an exception, it stays failing and goes to the owner with what was tried (the top of [AGENTS.md](../../AGENTS.md)). The owner decides an exception in person, rarely, and a contributor never does.

### The one exception

The owner allowed one on 2026-11-23: `clippy::float_arithmetic` in `crates/md2html/src/mermaid/geom.rs`, the geometry kernel of the diagrams.

A drawing's positions, widths, midpoints and the reach of a circle are floating-point by nature. Every sum, product and quotient of floating-point numbers the diagrams need is written in that one file, behind the type `Len` (a length in pixels) and a few helpers. The rest of the layout (`plane.rs` with `Point`, `Size` and `Rect`, the flowchart and the sequence diagram) adds, scales and compares lengths without touching the numbers inside. The file's module documentation says this, and its `#![expect]` carries the reason. Nothing else in md2html does floating-point arithmetic, and nothing else carries an exception.

The exception cannot outlive its cause: an `#[expect]` that no longer fires is reported by rustc (`unfulfilled_lint_expectations`), which `-D warnings` makes an error. The day the arithmetic leaves the file, the attribute has to go.

### `put!`

`put!` (in `crates/md2html/src/text.rs`) appends formatted text to a `String` and gives back no result. The owner decided to keep it: appending to a `String` cannot fail. An error could only come from a `Display` that reports one itself, and what it wrote stays. Without `put!`, every formatted write into the page would carry a result with nothing to do about it. It is the only such macro; everywhere else a write's result is returned or handled.

### Enforced by

- `allow_attributes = "deny"` in `Cargo.toml` refuses `#[allow]`; `allow_attributes_without_reason` refuses an attribute without a reason.
- `scripts/ci/anti_pattern_gate.sh` refuses `#[allow]` and `#[expect]` in any form (`#!`, `cfg_attr`, on a test module) in every Rust file git tracks. The one attribute it lets through is the approved one, in `geom.rs`, and only while it names `clippy::float_arithmetic` and nothing else. It has no marker that silences it. It runs in `just go` (the parallel validation stage), in `just pre-pr` (and alone as `just audit-gate`), and in CI (job `test` in `ci.yml`, on Linux).

## Where it is set

| Place | What it holds |
| --- | --- |
| `Cargo.toml`, `[workspace.lints.clippy]`, `[workspace.lints.rust]`, `[workspace.lints.rustdoc]` | Which lints run, at which level. Each line carries its reason. |
| `clippy.toml` | What a lint table cannot say: where a lint applies (tests) and Clippy's settings. |
| `[lints] workspace = true` in every crate | No crate has lints of its own. `md2html-manifest-audit` checks it (gate `manifest-drift`). |
| `just/shared.just`, `prod_flags` and `test_flags` | The strictest groups again, on the command line, so a mistake in `Cargo.toml` cannot quietly weaken the gates. |

## Levels

- The Clippy groups `pedantic`, `nursery` and `cargo` are at deny, whole.
- From the `restriction` group about a hundred lints are picked one by one, under headings in `Cargo.toml`. The whole group is not enabled: several of its lints contradict each other, and Clippy itself warns against it.
- rustc: `unsafe_code` is forbidden (md2html has no unsafe code and cannot gain any), `missing_docs` and `future_incompatible` deny, the style and hygiene lints warn.
- rustdoc: broken intra-doc links deny, the rest warn.
- Every gate runs with `-D warnings`, so a lint at warn stops a change as surely as one at deny.

Four lints are switched off in the table. That is a choice about the posture, made once for all code; no code is held to less than other code.

| Lint | Why it is off |
| --- | --- |
| `redundant_pub_crate` | It contradicts rustc's `unreachable_pub`, which is kept: an item in a private module is written `pub(crate)`. |
| `unneeded_field_pattern` | It contradicts `rest_pattern_accessible_field`, which is kept: a pattern names every field, and one that ignores them all is written `Foo { a: _, b: _ }`. |
| `single_call_fn` | A function called once is still a named part. It is a restriction lint and would be off anyway; the line makes the choice visible. |
| `multiple_crate_versions` | Duplicates are refused in one place, by `cargo deny` (`multiple-versions = "deny"` in `deny.toml`, gate `deny`), which names the crates that bring them; Clippy would only repeat it in every crate. `Cargo.lock` has none. md2html itself has no dependencies. |

Restriction lints deliberately not picked: `as_conversions` (the `cast_*` lints already refuse every lossy, wrapping or sign-changing `as`, and `cast_lossless` the ones a `From` can do), `arithmetic_side_effects` (it flags every `+`; md2html bounds its input instead, see [hostile_input_policy.md](hostile_input_policy.md)), `implicit_return` (it contradicts `needless_return`), and the purely stylistic `question_mark_used`, `pattern_type_mismatch`, `else_if_without_else` and `single_char_lifetime_names`.

## Tests and production code

`clippy.toml` lets tests use `unwrap`, `expect` and `panic!`: a test states what must hold and stops with the reason when it does not. Tests may not keep `dbg!` or print (`allow-dbg-in-tests` and `allow-print-in-tests` are false). Every other lint applies to tests exactly as to production code. The `lint-tests` gate repeats the same split on the command line (`test_flags`).

## The gates

| Gate | What it runs | When |
| --- | --- | --- |
| `lint-prod` | Clippy over libraries and binaries with `prod_flags` | pre-commit, pre-push |
| `lint-tests` | Clippy over the tests with `test_flags` | pre-commit, pre-push |
| `lint-ci` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pre-commit, pre-push, pull request (job `clippy`) |
| `lint-ci-no-default` | The same with `--no-default-features` | pre-push, pull request (job `clippy-no-default`) |
| `lint-ci-windows` | The same for `x86_64-pc-windows-msvc`, with cargo-xwin | pre-push, pull request (job `windows-lint`, native on Windows) |
| `lint-ci-linux-zig`, `lint-ci-mac-intel` | The same for Linux (cargo-zigbuild) and Intel macOS | pre-push |
| `msrv` | `cargo check` of md2html on the Rust its `rust-version` declares, with `-D warnings` as CI sets it | pre-push, and CI's `msrv` job (a build) |
| `rustdoc` | `cargo doc` with warnings as errors | pre-push, pull request (job `docs`) |

Every gate is required. A gate whose tool or target is missing fails, and names the command that installs it (`just install-dev-tools`). There is no field in `gates.toml` that makes a gate optional, and no variable that skips one. Two legs cannot run on every host, and say so: on x86_64 Linux the Linux leg is `lint-ci` itself, and away from macOS the Intel macOS leg has no Apple SDK to build against and prints `NOT CHECKED`.

Besides, `.github/workflows/ci.yml` runs `cargo clippy --release --all-targets` with `RUSTFLAGS=-D warnings` on macOS, Ubuntu and Windows for every pull request and every push to `main`, and the anti-pattern gate (see "The rule").

All gates are defined in `scripts/ci/gates.toml`; the hooks are generated from it (see "Keeping the hooks and CI in step").

## Fixing a finding

| Finding | Usual cause | Fix |
| --- | --- | --- |
| `indexing_slicing`, `string_slice`, `missing_asserts_for_indexing` | A position into text or a slice, kept by hand as a `usize` | Read with `crate::scan`: `Scan` over text, `Cursor` over a slice. They keep their own place and answer with an `Option` or a piece of the input. Or iterators, `split_once`, `strip_prefix`. Not a `.get()` at every old index. |
| `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, `cast_precision_loss` | A number held in a type that does not fit what it counts | Choose the type where the number is born. Where a conversion is real, `TryFrom` with the failure handled. |
| `float_arithmetic` | Coordinates and sizes as bare `f64` throughout a module (the diagram layout) | Hundreds of findings in one module say the quantity has no type. Give it one, with its arithmetic in one place: the diagrams' lengths are `mermaid::geom::Len`, and `geom.rs` is the one approved exception (see "The rule"). |
| `too_many_lines`, `cognitive_complexity` | One function doing several jobs | Named parts, each a function with its doc comment. |
| `too_many_arguments` | Inputs that travel together | A struct of them. |
| `struct_excessive_bools`, `fn_params_excessive_bools` | Flags of which only some combinations make sense | An enum for each real choice. The command line's choices are `Action` (write or check), `Mode` (once, watch, or serve on a port) and `Shape` (page or fragment) in `cli/args.rs`, not a flag each. |
| `option_option` | Three states in two `Option`s | An enum with three named variants: the sanitizer's `Verdict` on an attribute is `Value`, `Bare` or `Drop`. |
| `type_complexity`, tuples such as `(f64, f64, f64, f64)` | Data without a name | A named struct: a box on a drawing is `Rect { left, top, right, bottom }` (`mermaid/plane.rs`). |
| Parallel vectors indexed by one raw `usize` | Records split into columns | A vector of a record type; a newtype where a number must name an item (`scan::Mark`, `scan::Spot`, and the diagrams' `EdgeId` and `ActorId`, which a `mermaid::table::Table` is keyed by). |
| `shadow_reuse`, `shadow_same`, `shadow_unrelated` | One name for two things | A second name that says what changed. |
| `min_ident_chars` | One-letter names | The name of the thing. |
| `wildcard_enum_match_arm`, `rest_pattern_accessible_field` | A match that would accept a new variant or field without anyone deciding | Name every variant and field, so the compiler lists each place to decide. |
| `let_underscore_must_use`, `let_underscore_untyped`, `unused_result_ok`, `map_err_ignore` | A failure nobody decided about | Propagate it with `?`, turn it into a warning the reader sees, or handle each case in a `match`. |
| `print_stdout`, `print_stderr` | Parts of the program talking to the terminal | They return what they have to say (md2html's warnings are `render::Warning`, a line and a message); every line for a person goes through `Reporter` in `cli/output.rs`, on a locked handle, and a failed write ends the run with its own exit code. The tooling writes through `say!` (`scripts/ci-pipeline/src/out.rs`), which returns the write's error. |
| `infinite_loop` | A loop that never ends in a function whose type says it returns | Say so in the signature (`-> !`), or give the loop a way out. |
| `unwrap_used`, `expect_used`, `panic`, `exit` | See [panic_policy.md](panic_policy.md) | |
| `redundant_clone`, `format_push_string`, `string_add` and kin | See [allocation_policy.md](allocation_policy.md) | |

## The weekly checks

`.github/workflows/tier-2.yml` runs on Mondays at 06:00 UTC and on demand. It is slower than a pull request can wait for:

| Job | What it finds |
| --- | --- |
| `file-size-policy` | Rust files over 800 lines, every one git tracks, with no exception list (also gate `file-size` on every commit) |
| `coverage` | Line coverage of the test suite (`cargo llvm-cov nextest`) |
| `build-validation` | Every binary of the workspace builds from the lockfile, all features on |
| `udeps` | Unused dependencies, found by compiling (gate `machete` is the fast version) |
| `hack` | Each feature built on its own (`cargo hack --each-feature`) |
| `miri` | Undefined behaviour in the standard library as md2html's text-reading modules drive it: the unit tests of `block`, `config`, `export`, `highlight`, `inline`, `math`, `sanitize` and `text` |
| `careful` | The workspace's tests against a standard library built with debug assertions (`cargo careful test`) |
| `mutants` | Mutations of `sanitize.rs` and `text.rs` that the tests do not notice (`cargo mutants`, configured in `.cargo/mutants.toml`). The job is marked `continue-on-error`, so a surviving mutation does not yet fail the week's run. |

## Keeping the hooks and CI in step

`scripts/ci/gates.toml` is the one list of gates. `md2html-gen-hooks` writes `scripts/hooks/_lint_fast.sh` and `scripts/hooks/_lint_pre_push.sh` from it (gates `hooks-drift` and `fast-drift` refuse a hand edit), and `scripts/ci/check_gates_drift.sh` checks that every consumer names the same gates (gate `gates-drift`). `md2html-gen-workflow --check` (gate `workflow-drift`) reads the workflows against it and checks eight properties. Properties 1 to 4 read `pr-fast.yml`: every pull-request gate has a job, the job's `if:` matches the gate's `gate_when`, the `required` job needs every gate job, and no job exists without a gate. Properties 5 to 8 read every workflow under `.github/workflows`:

| # | Property | Source of truth |
| --- | --- | --- |
| 5 | One pin per action: every `uses: owner/name@<sha>` of an action names the same commit in all workflows | the workflows themselves |
| 6 | Tool versions: every `ziglang==<v>`, `cargo-zigbuild@<v>` and `cargo-vet@<v>` matches the manifest | `[toolchain]` in `gates.toml` |
| 7 | Build targets: a target's `RUSTFLAGS` match the manifest; `target-cpu=native` is refused anywhere | `[[target]]` in `gates.toml` |
| 8 | Test profiles: every `nextest --profile <name>` names a defined profile | `.config/nextest.toml` |

The tool reports drift and never rewrites a workflow.

## Changing the posture

- A lint is added, removed or given another level in `Cargo.toml` or `clippy.toml`, with its reason in the comment beside it, in a pull request of its own. The workspace is clean under the new table when it merges: there is nothing to park new findings in.
- Lowering a lint to get past a finding is not a change of posture; it is prohibited ([AGENTS.md](../../AGENTS.md), section 3).
- A toolchain update (`just toolchain-sync`) can bring new lints into the groups at deny. They are fixed in the same pull request.
- This document changes in the same pull request as the table it describes.
