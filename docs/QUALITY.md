<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# md2html against the definition of done

For a reviewer who wants to know whether this workspace is what it claims to be, and for whoever keeps it so: the thirteen questions a senior reviewer asks of a Rust workspace and the thirty lines of a definition of done, each answered against the code as it is, with the file, the test or the command that shows it. What is not done says why, and who decides.

Answered on 2026-11-24, at the end of an eighteen-phase review of the workspace (layering, visibility, types, errors, ownership, traits, features, build scripts, concurrency, platforms, performance, tests, documentation, lints and CI, releases, security). A change that makes an answer false changes this page in the same pull request; [GOTO.md](../GOTO.md) holds the state that moves faster.

## Check it yourself

The playbook's suite, from the workspace root (each exits 0; the last says "no reports are currently available", its clean answer, with exit 101):

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
cargo tree --workspace -d
cargo tree --workspace -e features
cargo build --workspace --timings
cargo report future-incompatibilities
```

md2html's own, on top: `just go` (the definition of done for a change), `just lint-pre-push` (the push gates, the minimum Rust and the Windows, Linux and Intel Mac lint legs among them), `just epubcheck` (the repository's books through the W3C's validator), `just differential` (8,300 documents rendered six ways, byte for byte against `origin/main`), the browser suite in `tests/browser` (`node audit.js`, `node interact.js`, `node live.js`, `node nesting.js`), and `md2html --check README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md`. [TESTING.md](TESTING.md) says what each guards and where it runs.

Results on 2026-11-24: every command above green; 652 of 652 CommonMark examples (`src/spec_tests.rs`, a floor); 573 tests in the workspace's targets (602 with every feature on), 26 doc tests; 20 books with no epubcheck message, and none in a seeded sample of 240 of the differential corpus's documents made as books with `--html`; of the corpus's 49,830 renders, 180 differ from `main`'s, each one of that day's fixes: 57 pages (19 documents with an unreadable formula in a line, in three page modes) and 123 books (an attribute XHTML does not define 88, the formula 19, a `div` in text 8, a link to an anchor the book lacks 5, a link to a host no reader can look up 5, an image without a source 3, ruby 2; some books more than one).

## The reviewer's thirteen questions

| Question | Answer | Evidence |
| --- | --- | --- |
| Is the crate graph easy to explain? | Yes | Seven crates: the product and six tools that never meet it ([ARCHITECTURE.md](ARCHITECTURE.md), "The workspace's crates"). md2html has no dependency; the tooling's 28 crates appear once each (`cargo tree --workspace -d` lists only the workspace's own `md2html-version`, a normal and a build dependency of one version). |
| Is there a clear dependency direction? | Yes | md2html's 21 modules stand in nine layers, each using its own or a lower one, none in a circle ([ARCHITECTURE.md](ARCHITECTURE.md)); `modules_keep_to_their_layers` and `every_module_root_is_its_storefront` in `crates/md2html/src/layer_tests.rs` read every production file and fail on an upward use, a circle or a submodule reached past its root. The review started from three circles. |
| Is the public API intentionally curated? | Yes | md2html is a binary: its contract is the command line, written in [command-line.md](command-line.md) and held to the code by `src/cli/contract_tests.rs` (options, settings, exit codes, environment variables). Inside it no item is `pub`; each is private, `pub(super)` or `pub(crate)` as far as its users and no further (`unreachable_pub` with `-D warnings`), submodules are private behind their module's root. The libraries offer what their binaries use: `md2html-version` two items, the dev-tools library 22. |
| Are most invariants enforced by types? | Yes | [type_policy.md](policies/type_policy.md), "Types": `text::Html` for markup, `model::Level` one to six, `scan::Mark`/`Spot` for places in text, id types in the diagrams, `mermaid::geom::Len`, settings read once into their types, enums where a boolean picked a way of working; `partial_pub_fields`, `wildcard_enum_match_arm` and `rest_pattern_accessible_field` at deny in the root `Cargo.toml`. |
| Are error types consistent and informative? | Yes | One strategy per kind of crate ([panic_policy.md](policies/panic_policy.md), "Errors, by kind of crate"): md2html a message naming the file and the cause and a documented exit code (`exit_codes_keep_their_numbers` in `src/report.rs`), `anyhow` with context in the tool binaries, typed errors with a source in the two libraries. |
| Are panics rare and justified? | Yes | None in production code: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` at deny; 0 `unwrap`, `expect`, panic macros and asserts outside tests in every crate. |
| Is async/concurrency code explicit and reviewable? | Yes | No async. One threaded part, `--serve`, fits on a page ([concurrency_policy.md](policies/concurrency_policy.md)): every thread has an owner that waits for it, the shared state is one `Site`; tests over real connections hold its limits and its stop (`src/serve/tests/tcp.rs`). |
| Is unsafe isolated and documented? | Yes | There is none: `unsafe_code = "forbid"` for every crate; no FFI ([platform_policy.md](policies/platform_policy.md)). |
| Are public items documented with examples where useful? | Yes | `missing_docs` and `missing_docs_in_private_items` at deny: every item says what it is; `cargo doc` with private items is clean under `-D warnings`; 26 doc tests in the libraries. md2html's examples are its documents ([capabilities.md](capabilities.md)) and the tests that run the built program (`crates/md2html/tests/command_line/`). |
| Does CI enforce quality automatically? | Yes | 39 gates in one manifest (`scripts/ci/gates.toml`): 12 at commit, 36 at push, 29 on a pull request, generated into the hooks and checked against the workflow by drift gates; the ruleset requires the checks and signed commits (`scripts/ci/main-ruleset.json`); the weekly tier-2 runs Miri, cargo-careful and mutation testing ([TESTING.md](TESTING.md)). |
| Are features understandable and tested? | Yes | One feature in the workspace, `build` in `md2html-version`, off by default; built, linted and tested with it on and off (gates `lint-ci`, `lint-ci-no-default`, the weekly feature matrix) ([dependency_policy.md](policies/dependency_policy.md)). |
| Is MSRV/edition policy explicit? | Yes | Edition 2024 for the workspace (root `Cargo.toml`); md2html's minimum Rust is 1.85 (`crates/md2html/Cargo.toml`), with when and how it may rise in [release_policy.md](policies/release_policy.md), "The minimum Rust"; the `msrv` gate and CI job build md2html with it. One test does not pass on it: see below. |
| Are performance and build-time hotspots measured rather than guessed? | Yes | `just measure` times the stages of a render (`src/tests/measure.rs`); [allocation_policy.md](policies/allocation_policy.md), "Measured, not guessed", names the numbers each change moved; tests hold time and memory by size where they can (`src/tests/hostile.rs`, `src/block/lines/tests.rs`, `src/inline/tests.rs`). Build: 6 s for the workspace from clean, md2html's release build (LTO, one codegen unit) 15 to 27 s, `cargo build --timings` for the detail. |

## The definition of done

| Line | Answer | Evidence |
| --- | --- | --- |
| **Architecture** | | |
| Every crate has a single obvious responsibility | Yes | [ARCHITECTURE.md](ARCHITECTURE.md), the crates' table; why md2html stays one crate is its last paragraph. |
| Dependency direction is documented and followed | Yes | The layers above; `src/layer_tests.rs`. |
| No junk-drawer crate remains | Yes | No `utils` or `common`; a helper two layers share lives in the lower one, named for what it does (`text`, `scan`, `system`). |
| Public boundary crates are separated from internal implementation crates | Yes | No crate is published (`publish = false`); the product and the tooling never depend on each other. |
| **API design** | | |
| Public surface is intentionally curated | Yes | No `pub` item in md2html; the contract is the command line (above). |
| Internal modules are hidden unless intentionally public | Yes | Every module private; a module's root is its one door (`every_module_root_is_its_storefront`). |
| Re-exports are used deliberately to create clean paths | Yes | A module's root re-exports what its submodules offer the rest (`config.rs` offers `Settings`); the crate root re-exports nothing ([ARCHITECTURE.md](ARCHITECTURE.md), "Directions"). |
| Public fields are rare and justified | Yes | No `pub` field in md2html, four in the workspace (one record, `Platform` in `scripts/dev-tools/src/platforms.rs`). Inside md2html a struct shows its fields only when it is a record of named data whose fields do not constrain one another, and all or none of them (`partial_pub_fields`); a value with a rule keeps them private ([type_policy.md](policies/type_policy.md)). |
| Invariants are enforced by constructors/types | Yes | [type_policy.md](policies/type_policy.md), "Types". |
| **Errors and safety** | | |
| Library APIs avoid stringly-typed errors | Yes | The two libraries return typed errors; md2html, a binary, returns a message where an error is only ever shown, by a written rule ([panic_policy.md](policies/panic_policy.md), "Errors in md2html"). |
| Panic sites are rare and justified | Yes | None in production code (above). |
| Unsafe code is isolated and documented | Yes | Forbidden. |
| Platform and FFI assumptions are explicit | Yes | No FFI; no `cfg` on an operating system in md2html; what differs between systems is one module (`system`) with a test per system where CI runs it ([platform_policy.md](policies/platform_policy.md)). |
| **Dependencies and features** | | |
| Duplicate dependency pain is reduced | Yes | No crate in two versions; `cargo deny check` bans and licences clean ([SUPPLY-CHAIN.md](SUPPLY-CHAIN.md)). |
| Default features are lean | Yes | One feature, off by default. |
| Public features are documented and tested | Yes | Above. |
| Patch/override usage is documented | Yes | None in use: no `[patch]`, no `[replace]`, no git source. |
| **Tests and docs** | | |
| Public behavior has integration-style coverage | Yes | The built program run as a person runs it (`crates/md2html/tests/command_line/`), the differential test, generated documents with their invariants, the browser suite in Chrome, every book through epubcheck ([TESTING.md](TESTING.md)). |
| Important docs examples compile | Yes | 26 doc tests pass. |
| Crate-level docs exist where needed | Yes | Every crate root and every module (`missing_docs`). |
| Broken intra-doc links are eliminated | Yes | `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --document-private-items` is clean (gate `rustdoc`); the documents' own paths, recipes and gates by gate `doc-references`. |
| **Tooling and CI** | | |
| Workspace lint inheritance is configured consistently | Yes | Every crate inherits `[workspace.lints]` (gate `manifest-drift`); one approved exception in all of the code, written down with its reason ([lint-posture.md](policies/lint-posture.md)). |
| Formatting is standardized | Yes | `rustfmt.toml`, gate `fmt`. |
| CI denies warnings through Clippy or equivalent policy | Yes | `-D warnings` in every clippy gate, on four platforms. |
| Stable toolchain checks are green | Yes, with one exception below | The workspace's tools run on a pinned nightly by choice (`rust-toolchain.toml`: one toolchain for every environment, refreshed by `just toolchain-sync`). md2html is checked and built with Rust 1.85, its minimum, with `-D warnings` on every pull request (`msrv`); its tests pass on 1.85 and on the current stable, 1.100, but one. |
| Optional MSRV/nightly advisory jobs exist if policy requires them | Yes | `msrv` (required) and the weekly nightly canary. |
| **Release discipline** | | |
| `rust-version` / MSRV policy is explicit | Yes | [release_policy.md](policies/release_policy.md), "The minimum Rust". |
| Edition policy is explicit | Yes | 2024, in `[workspace.package]`. |
| Semver expectations are documented for public crates | Yes | Nothing is published; what a version number promises for the command line is [release_policy.md](policies/release_policy.md), "What a version number promises". |
| Release checklist exists | Yes | [release_policy.md](policies/release_policy.md), "The release checklist", with what only a real run of the release workflow proves; the release lane has not run yet ([GOTO.md](../GOTO.md), section 5). |

## Not done, and why

- **A closed standard output, with md2html built by a stable Rust.** md2html ends with 74 when it cannot write its summary to standard output ([command-line.md](command-line.md)), and `a_closed_standard_output_ends_with_74_or_73` holds it, on the pinned toolchain. The standard library of Rust 1.85 to 1.100, the current stable, takes a write to a closed or read-only standard output (`EBADF`) for a success (the nightly md2html is pinned to no longer does), so md2html built with them, as `cargo install --git` on a stable toolchain builds it, ends with 0 there; the test fails on them. md2html cannot see the error through the standard library on those versions without `cfg` on an operating system, which [platform_policy.md](policies/platform_policy.md) keeps out of md2html, or `unsafe`, which is forbidden. The way out (raise the minimum Rust to the first stable release that carries the change, a platform rule in `system`, or a note in the command line's contract) is the owner's to decide, with running the tests in the `msrv` job.
- **Memory of lists nested deep on every line.** A line of thirty list markers (`- - - ... a`) is thirty list items in twenty-nine lists: about 130 bytes of tree each, near the reference implementation's node, so about 75 bytes for each byte of such input, bounded by the nesting limit of 64 (64 MB of it: 5.4 GB for the page, 17 s). The time had causes and they are fixed; the memory is the tree. A smaller one is an arena of nodes, a change of the model that every reader and writer of it would follow, for a shape no document has.
- **The owner's open questions** ([GOTO.md](../GOTO.md), section 1, "Pending").
