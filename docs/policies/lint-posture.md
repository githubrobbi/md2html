<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# The lint posture

What the compiler, Clippy and rustdoc check in this workspace, where that is set, which gate runs it, and what to do when a lint fires: the first policy to read for whoever writes code here. The other documents in this folder give the reasoning for particular groups of lints:

- [panic_policy.md](panic_policy.md): production code returns errors, it does not panic.
- [allocation_policy.md](allocation_policy.md): borrow before copying, write into one string.
- [type_policy.md](type_policy.md): data gets a type that says what it is.
- [concurrency_policy.md](concurrency_policy.md): the one place with threads.
- [hostile_input_policy.md](hostile_input_policy.md): what the compiler does not catch in a tool that reads other people's text.
- [platform_policy.md](platform_policy.md): no unsafe code and no foreign function; what differs between Linux, macOS and Windows, where it is handled and which test holds it on which system.
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
- `scripts/ci/anti_pattern_gate.sh` refuses `#[allow]` and `#[expect]` in any form (`#!`, `cfg_attr`, on a test module) in every Rust file git tracks. The one attribute it lets through is the approved one, in `geom.rs`, and only while it names `clippy::float_arithmetic` and nothing else. It has no marker that silences it. It is gate `anti-pattern`: it runs at every commit and every push (the hooks), on every pull request (job `anti-pattern`), and in `just go` (and alone as `just audit-gate`). Gate `gate-tests` holds the gate itself to its rules: a planted violation of each is refused (an attribute with blanks inside, a `cfg_attr` over several lines, the approved lint in another file, a second lint beside it, an `allow` in its place), the approved attribute passes, and a checkout with no Rust file is an error, never a pass.

## Where it is set

| Place | What it holds |
| --- | --- |
| `Cargo.toml`, `[workspace.lints.clippy]`, `[workspace.lints.rust]`, `[workspace.lints.rustdoc]` | Which lints run, at which level. Each line carries its reason. |
| `clippy.toml` | What a lint table cannot say: where a lint applies (tests) and Clippy's settings. |
| `[lints] workspace = true` in every crate | No crate has lints of its own. `md2html-manifest-audit` checks it (gate `manifest-drift`). |
| `just/shared.just`, `lint_flags` | The three groups again at deny, on the command line, so a mistake in `Cargo.toml` cannot quietly weaken the gates; and the two lints the table switches off inside them. Nothing on the command line is lower than the table. `just go` and `just ship` run the same recipes (`lint-prod`, `lint-tests`), not a copy of the flags. |

## Levels

- The Clippy groups `pedantic`, `nursery` and `cargo` are at deny, whole.
- From the `restriction` group 107 of its 137 lints are picked one by one, under headings in `Cargo.toml`. The whole group is not enabled: several of its lints contradict each other, and Clippy itself warns against it. Each of the other 30 has its reason below.
- rustc: `unsafe_code` is forbidden (md2html has no unsafe code and cannot gain any); `missing_docs`, `future_incompatible`, the unused-code hygiene lints (`unused_import_braces`, `unused_lifetimes`, `unused_macro_rules`, `unused_qualifications`), the unsafe lints and fourteen allow-by-default lints picked from rustc's list deny; the style lints warn.
- rustdoc: broken intra-doc links deny, the rest warn, every rustdoc lint named.
- Every gate runs with `-D warnings`, so a lint at warn stops a change as surely as one at deny.

Four lints are switched off in the table. That is a choice about the posture, made once for all code; no code is held to less than other code.

| Lint | Why it is off |
| --- | --- |
| `redundant_pub_crate` | It contradicts rustc's `unreachable_pub`, which is kept: an item in a private module is written `pub(crate)`. |
| `unneeded_field_pattern` | It contradicts `rest_pattern_accessible_field`, which is kept: a pattern names every field, and one that ignores them all is written `Foo { a: _, b: _ }`. |
| `single_call_fn` | A function called once is still a named part. It is a restriction lint and would be off anyway; the line makes the choice visible. |
| `multiple_crate_versions` | Duplicates are refused in one place, by `cargo deny` (`multiple-versions = "deny"` in `deny.toml`, gate `deny`), which names the crates that bring them; Clippy would only repeat it in every crate. `Cargo.lock` has none. md2html itself has no dependencies. |

### Lints the table does not name, and why

Every lint the toolchain offers was read against the table (2026-11, `nightly-2026-11-23`: clippy 0.1.102, rustc 1.102), with the number of findings each would have over the workspace. Those with no finding, or a few fixed at their cause, are switched on (the comment in `Cargo.toml` says so). The rest stay off for the reason given here; the count is the findings at that reading. A toolchain update repeats the reading for the lints it adds (see "Changing the posture").

Restriction lints that contradict a lint the table keeps:

| Lint | Findings | Why it is off |
| --- | --- | --- |
| `implicit_return` | 5434 | Contradicts `needless_return`: a block ends in its value. |
| `pub_with_shorthand` | 1118 | Contradicts `pub_without_shorthand` (kept): `pub(crate)`, not `pub(in crate)`. |
| `field_scoped_visibility_modifiers` | 868 | Contradicts `unreachable_pub` (kept): a field of a type in a private module is `pub(crate)`. |
| `separated_literal_suffix` | 552 | Contradicts `unseparated_literal_suffix` (kept): `1_u8`. |
| `self_named_module_files` | 55 | Contradicts `mod_module_files` (kept): a module with children is a file beside its folder (`config.rs` and `config/`), never a file named mod.rs inside it. |
| `inline_asm_x86_att_syntax`, `inline_asm_x86_intel_syntax` | 0 | A pair that refuse each other; and `unsafe_code` is forbidden, so there is no assembly. |

Restriction lints that would refuse what this code does on purpose:

| Lint | Findings | Why it is off |
| --- | --- | --- |
| `non_ascii_literal` | 2495 | md2html writes typographic quotes, dashes and the words of seventeen languages; text is Unicode by design. |
| `question_mark_used` | 1602 | `?` is how an error travels here ([panic_policy.md](panic_policy.md)). |
| `arbitrary_source_item_ordering` | 1606 | Items are ordered for the reader (what a module is for first, helpers after), not by kind. |
| `absolute_paths` | 842 | A path written once (`std::fs::write`) reads better than an import used once; `unused_qualifications` (kept) refuses the redundant ones. |
| `arithmetic_side_effects` | 644 | It flags every `+`; md2html bounds its input instead ([hostile_input_policy.md](hostile_input_policy.md)). |
| `integer_division`, `integer_division_remainder_used` | 90, 157 | Columns, widths and bytes are whole numbers; dividing them whole is the meaning, not an accident. |
| `big_endian_bytes`, `little_endian_bytes` | 19, 8 | Formats fix their byte order (PNG big-endian, ZIP little-endian), so the explicit call is the correct one; `host_endian_bytes` (kept) refuses the one that varies. |
| `multiple_inherent_impl` | 26 | A type's methods are split by concern across the files of its module. |
| `must_use_without_reason` | 112 | `must_use_candidate` (pedantic) marks every pure function; a reason on each would repeat "the result is the point". |
| `pattern_type_mismatch`, `else_if_without_else`, `single_char_lifetime_names`, `inline_trait_bounds` | 260, 32, 19, 17 | Style only, against the style rustfmt and the code follow. |
| `inline_modules` | 0 | It skips `mod tests { }`, which is what the code has; switching it on would add nothing but a rule against a future small module. |

Restriction lints for libraries published to others (md2html publishes nothing; its tooling's two libraries are used by the workspace alone):

| Lint | Findings | Why it is off |
| --- | --- | --- |
| `exhaustive_enums`, `exhaustive_structs` | 4, 2 | `#[non_exhaustive]` protects downstream users; there are none. |
| `missing_inline_in_public_items` | 50 | Cross-crate inlining of a library nobody else links. |
| `missing_trait_methods` | 60 | It asks every impl to spell out the trait's provided methods. |
| `pub_use` | 8 | `md2html-dev-tools` keeps its modules private and re-exports what its two binaries use from one root, as its `lib.rs` says. |
| `doc_paragraphs_missing_punctuation` | 160 (80 places) | Most are the word tables of `config/words.rs`, whose doc is the English word itself (`/// "Skip to content"`), not a sentence. |
| `decimal_literal_representation` | 0 | It asks for hex where a decimal number is the meaning (a size, a count). |

rustc's allow-by-default lints not picked: `unused_results` (528 findings: it flags every ignored value, the previous entry `insert` returns among them; a result that matters is `#[must_use]`, and `unused_must_use` is on), `dead_code_pub_in_binary` (6 findings, all false: it reads a library's public items in its test harness, a binary, as unused; md2html's own `pub` items are held by `unreachable_pub`; and md2html's minimum Rust does not know it), `missing_copy_implementations` (no finding; `Copy` is a promise made type by type, and `trivially_copy_pass_by_ref` covers what it would buy). The edition-migration lints (`rust_2021_*`, `keyword_idents_*`, `absolute_paths_not_starting_with_crate`, `macro_use_extern_crate`, and the 2024 ones, which `rust_2024_compatibility` turns on as a group) concern code older than edition 2024. `implicit_provenance_casts`, `must_not_suspend`, `multiple_supertrait_upcastable`, `non_exhaustive_omitted_patterns`, `unqualified_local_imports`, `resolving_to_items_shadowing_supertrait_items`, `shadowing_supertrait_items`, `unmarked_unchecked_type_aliases` and `unstable_features` exist only behind a nightly feature. The unsafe and foreign-function ones (`ffi_unwind_calls`, `missing_unsafe_on_extern`, `unsafe_attr_outside_unsafe`, `deprecated_safe_2024`, `deprecated_llvm_intrinsic`) cannot fire while `unsafe_code` is forbidden, and `closure_returning_async_block`, `deref_into_dyn_supertrait` and `linker_info` concern code md2html does not have. rustdoc's `missing_doc_code_examples` exists only behind a nightly feature.

### On the oldest Rust

The `msrv` gate and CI's `msrv` job build md2html with its minimum Rust (`rust-version`, [release_policy.md](release_policy.md)) and `-D warnings`. A lint the table names that is newer than that Rust (`raw_borrows_via_references` today) is unknown there: rustc says so with a warning (E0602) that `-D warnings` does not turn into an error for lints given on the command line, which is what Cargo's `[lints]` does. That warning is the only one the lane prints; every lint picked from rustc's list above is known to it.

## Tests and production code

`clippy.toml` lets tests use `unwrap`, `expect` and `panic!`: a test states what must hold and stops with the reason when it does not. Tests may not keep `dbg!` or print (`allow-dbg-in-tests` and `allow-print-in-tests` are false). Every other lint applies to tests exactly as to production code. No command-line flag adds to the split: `lint-tests` runs with the same `lint_flags` as `lint-prod`.

## The gates

| Gate | What it runs | When |
| --- | --- | --- |
| `lint-prod` | Clippy over libraries and binaries with `lint_flags` | pre-commit, pre-push |
| `lint-tests` | Clippy over the tests with `lint_flags` | pre-commit, pre-push |
| `lint-ci` | `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings` (CI adds `--locked`) | pre-commit, pre-push, pull request (job `clippy`) |
| `lint-ci-no-default` | The same with `--no-default-features` | pre-push, pull request (job `clippy-no-default`) |
| `lint-ci-windows` | The same for `x86_64-pc-windows-msvc`, with cargo-xwin | pre-push, pull request (job `windows-lint`, native on Windows) |
| `lint-ci-linux-zig`, `lint-ci-mac-intel` | The same for Linux (cargo-zigbuild) and Intel macOS | pre-push |
| `msrv` | `cargo check` of md2html on the Rust its `rust-version` declares, with `-D warnings` as CI sets it | pre-push, and CI's `msrv` job (a build) |
| `rustdoc` | `cargo doc` with warnings as errors | pre-push, pull request (job `docs`) |

Every gate is required. A gate whose tool or target is missing fails, and names the command that installs it (`just install-dev-tools`). There is no field in `gates.toml` that makes a gate optional, and no variable that skips one. Two legs cannot run on every host, and say so: on x86_64 Linux the Linux leg is `lint-ci` itself, and away from macOS the Intel macOS leg has no Apple SDK to build against and prints `NOT CHECKED`.

Besides, `.github/workflows/ci.yml` runs md2html's `cargo clippy --release --all-targets --locked` (the workspace's default member) with `RUSTFLAGS=-D warnings` on macOS, Ubuntu and Windows for every pull request and every push to `main`, and the anti-pattern gate (see "The rule").

All gates are defined in `scripts/ci/gates.toml`; the hooks are generated from it (see "Keeping the hooks and CI in step").

## Fixing a finding

| Finding | Usual cause | Fix |
| --- | --- | --- |
| `indexing_slicing`, `string_slice`, `missing_asserts_for_indexing` | A position into text or a slice, kept by hand as a `usize` | Read with `crate::scan`: `Scan` over text, `Cursor` over a slice. They keep their own place and answer with an `Option` or a piece of the input. Or iterators, `split_once`, `strip_prefix`. Not a `.get()` at every old index. |
| `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, `cast_precision_loss` | A number held in a type that does not fit what it counts | Choose the type where the number is born. Where a conversion is real, `TryFrom` with the failure handled. |
| `float_arithmetic` | Coordinates and sizes as bare `f64` throughout a module (the diagram layout) | Hundreds of findings in one module say the quantity has no type. Give it one, with its arithmetic in one place: the diagrams' lengths are `mermaid::geom::Len`, and `geom.rs` is the one approved exception (see "The rule"). |
| `too_many_lines`, `cognitive_complexity` | One function doing several jobs | Named parts, each a function with its doc comment. |
| `too_many_arguments` | Inputs that travel together | A struct of them. |
| `struct_excessive_bools`, `fn_params_excessive_bools` | Flags of which only some combinations make sense | An enum for each real choice. The command line's choices are `Action` (write or check) and `Mode` (once, watch, or serve on a port) in `cli/args.rs` and `Shape` (page or fragment) in `document.rs`, not a flag each. |
| `option_option` | Three states in two `Option`s | An enum with three named variants: the sanitizer's `Verdict` on an attribute is `Value`, `Bare` or `Drop`. |
| `type_complexity`, tuples such as `(f64, f64, f64, f64)` | Data without a name | A named struct: a box on a drawing is `Rect { left, top, right, bottom }` (`mermaid/plane.rs`). |
| Parallel vectors indexed by one raw `usize` | Records split into columns | A vector of a record type; a newtype where a number must name an item (`scan::Mark`, `scan::Spot`, and the diagrams' `EdgeId` and `ActorId`, which a `mermaid::table::Table` is keyed by). |
| `shadow_reuse`, `shadow_same`, `shadow_unrelated` | One name for two things | A second name that says what changed. |
| `min_ident_chars` | One-letter names | The name of the thing. |
| `wildcard_enum_match_arm`, `rest_pattern_accessible_field` | A match that would accept a new variant or field without anyone deciding | Name every variant and field, so the compiler lists each place to decide. |
| `let_underscore_must_use`, `let_underscore_untyped`, `unused_result_ok`, `map_err_ignore` | A failure nobody decided about | Propagate it with `?`, turn it into a warning the reader sees, or handle each case in a `match`. |
| `print_stdout`, `print_stderr` | Parts of the program talking to the terminal | They return what they have to say (md2html's warnings are `render::Warning`, a line and a message); every line for a person goes through `Reporter` in `report.rs`, on a locked handle, and a failed write ends the run with its own exit code. The tooling writes through `say!` (`scripts/ci-pipeline/src/out.rs`), which returns the write's error. |
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
| `miri` | Undefined behaviour in the standard library as md2html's text-reading modules drive it: the unit tests of `block`, `config`, `export` (its `book`, `images`, `xhtml` and `zip`; not `print`, which starts Chrome), `highlight`, `inline`, `math`, `sanitize`, `scan` and `text` |
| `careful` | The workspace's tests against a standard library built with debug assertions (`cargo careful test`) |
| `mutants` | Mutations of the modules listed in `scripts/ci/mutants-scope.txt` (the sanitizer, the text helpers, the inline parser, the block reader and the document assembly) that the tests do not notice (`cargo mutants`; its settings in `.cargo/mutants.toml`). Every mutant there is caught (`docs/TESTING.md` has the counts), so one that escapes fails the week's run; a mutant that loops forever times out and counts as caught. The scope widens a module at a time, as each module's tests catch every mutant. |

## Shell, workflows and formats

The same rule holds outside Rust:

| Gate | What it reads | Fix |
| --- | --- | --- |
| `shellcheck` | Every shell script git tracks and the body of every `just` recipe (`scripts/ci/check_shell.py`); a `# shellcheck disable=` directive and a recipe line that ignores its own failure (`-`) are findings too | The script or recipe line it names |
| `actionlint` | Every workflow, and the shell of its `run:` blocks | The workflow it names |
| `anti-pattern`, `gate-tests` | See "Enforced by" | |
| `fmt`, `taplo`, `vet-fmt` | The formats of Rust (`rustfmt.toml`), TOML (`.taplo.toml`) and the supply-chain store (`cargo vet fmt`) | `just fmt` formats all three; `just fmt-check` checks them |

`.editorconfig` tells editors the rest (UTF-8, LF, final newline, indentation per kind of file); `.gitattributes` keeps line endings LF in the repository.

The workflows were also read with `zizmor` (1.33.1, at its default persona, offline and online), and every finding it has a fix for is fixed: tokens not kept by a checkout that does not push, no `${{ }}` inside a shell, write permissions on the jobs that need them, no cache restored where a release is built. Three remain, each a design with no alternative today: the `workflow_run` trigger of `auto-rerun-transient.yml` and `ci-failure-notify.yml` (nothing else runs after another workflow ends; both read no code of the pull request and pass every event value through `env:`), and the local reusable-workflow call of `dependabot-auto-merge.yml`, whose spelling zizmor asks for (`$/`) actionlint does not accept yet. So zizmor is not a gate: a gate that cannot be green without an exception would need one. It is run by hand at each workflow change until those three have an answer.

## Keeping the hooks and CI in step

`scripts/ci/gates.toml` is the one list of gates. `md2html-gen-hooks` writes `scripts/hooks/_lint_fast.sh` and `scripts/hooks/_lint_pre_push.sh` from it (gates `hooks-drift` and `fast-drift` refuse a hand edit), and `scripts/ci/check_gates_drift.sh` checks that every consumer names the same gates (gate `gates-drift`). `md2html-gen-workflow --check` (gate `workflow-drift`) reads the workflows against it and checks ten properties. Properties 1 to 4 read `pr-fast.yml`: every pull-request gate has a job, the job's `if:` runs at least whenever its gates' `gate_when` says, the `required` job needs every gate job, and that job is named `PR Fast CI / required`, the name branch protection requires (a job without a gate is `gates-drift`'s to find). Properties 5 to 8 read every workflow under `.github/workflows`:

| # | Property | Source of truth |
| --- | --- | --- |
| 5 | One pin per action: every `uses: owner/name@<sha>` of an action names the same commit in all workflows | the workflows themselves |
| 6 | Tool versions: every `ziglang==<v>`, `cargo-zigbuild@<v>` and `cargo-vet@<v>` matches the manifest | `[toolchain]` in `gates.toml` |
| 7 | Build targets: a target's `RUSTFLAGS` match the manifest; `target-cpu=native` is refused anywhere | `[[target]]` in `gates.toml` |
| 8 | Test profiles: every `nextest --profile <name>` names a defined profile | `.config/nextest.toml` |
| 9 | Change classes: `pr-fast.yml`'s `classify` job computes rust, dep and infra with the patterns the hooks are generated from | `[classification]` in `gates.toml` |
| 10 | Commands: each pull-request gate's job has a `run:` line that is the gate's `command` (placeholders and a leading `env` aside), or the `ci_command` the gate declares where CI runs it otherwise | `command` and `ci_command` of each `[[gate]]` |

The tool reports drift and never rewrites a workflow.

## Changing the posture

- A lint is added, removed or given another level in `Cargo.toml` or `clippy.toml`, with its reason in the comment beside it, in a pull request of its own. The workspace is clean under the new table when it merges: there is nothing to park new findings in.
- Lowering a lint to get past a finding is not a change of posture; it is prohibited ([AGENTS.md](../../AGENTS.md), section 3).
- A toolchain update (`just toolchain-sync`) can bring new lints into the groups at deny. They are fixed in the same pull request. It can also bring lints into the `restriction` group or rustc's allow-by-default list, which nothing switches on by itself: each is read as in "Lints the table does not name" and picked, or written into that section with its reason.
- This document changes in the same pull request as the table it describes.
