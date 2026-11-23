<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Panics and errors

## The rule

Production code does not panic. A failure is a value its caller sees: functions return `Result` or `Option`, and the binary turns an error into a message on standard error and an exit status. Tests may `unwrap`, `expect` and `panic!`, because there stopping with the reason is how a test says what must hold.

## Why

md2html reads text that someone else may have written. A panic on such text is a crash that the author of the text chose, and SECURITY.md promises that hostile input cannot crash md2html. A panic also takes the decision from the caller: `--check` wants to report every problem in a document, and a render wants to go on and say what it skipped. Only the caller knows which.

Indexing and slicing belong to the same family. `text[a..b]` panics when `b` is past the end or not on a character boundary, and in a parser every position is a chance to be wrong. A reader that keeps its own place (`crate::scan`) has no position to get wrong.

## Enforced by

- `Cargo.toml`, at deny: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `get_unwrap`, `unwrap_in_result`, `indexing_slicing`, `string_slice`, `missing_asserts_for_indexing`, `exit`, and from `pedantic` `missing_panics_doc`.
- `clippy.toml`: `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`.
- Gates `lint-prod`, `lint-tests`, `lint-ci` ([lint-posture.md](lint-posture.md)).
- The tests that render pathological inputs without a crash ([hostile_input_policy.md](hostile_input_policy.md)).

## What to write instead

| Instead of | Write |
| --- | --- |
| `.unwrap()` on an `Option` that may be empty | `let Some(x) = ... else { ... }`, `?` in a function that returns `Option`, or `map_or` / `unwrap_or_default` where the default is the right answer and not a way to hide the case |
| `.expect("cannot fail")` on an invariant | The invariant as a type: a value that is always there is not an `Option`; a place in the text comes from `Scan::mark`, so it is always inside the text and on a character boundary |
| `text[a..b]`, `items[i]` | `Scan` or `Cursor` from `crate::scan`, iterators, `split_once`, `strip_prefix` |
| `panic!` on a case that cannot happen | An enum whose variants are exactly the cases that can, matched without a wildcard |
| `std::process::exit(n)` | `main` returns `std::process::ExitCode` (`fn main() -> ExitCode { cli::main() }`); the process ends in one place, with a code from `cli::Exit` |
| A failure dropped with `let _ =` or `.ok()` | `?`, a warning the reader sees, or a `match` that handles each case |

## Errors in md2html

md2html is one binary, and almost every error ends as one line for the person who ran it. A function whose error is only ever shown returns `Result<T, String>` with a message that names the file and the problem (`Settings::set` in `config/settings.rs` for a front matter key, `megabytes` in `cli/args.rs` for a size on the command line). When a caller must act differently on different failures, the error is an enum with a variant for each: `input::Unread` is `TooLarge`, `NotText` or `Failed`, and the caller adds which option raises the limit.

A run that stops is a `Failure` (in `cli/output.rs`): the message and an `Exit`, an enum whose variants are the ways a run ends. Their numbers follow `sysexits.h`:

| Exit | Code | When |
| --- | --- | --- |
| `Done` | 0 | Everything asked for was done |
| `Findings` | 1 | `--check` found something to fix |
| `Usage` | 64 | The command line was wrong, or asked for something md2html refuses |
| `NoInput` | 66 | An input could not be read, or there is none any more |
| `Unavailable` | 69 | No port was free for `--serve` |
| `Software` | 70 | A PDF could not be printed, or printing left something behind |
| `CantCreate` | 73 | A page or a book could not be written |
| `Io` | 74 | A message, or the server's listener, failed |

Every line for a person goes through `Reporter`, on a locked handle. A write that fails (the reader closed the pipe) does not panic and is not lost: the run ends with `Io` unless it already failed for a reason of its own. The test `exit_codes_keep_their_numbers` (`src/cli/tests.rs`) holds the numbers.

Problems in a document that do not stop the page are warnings: a message, with the line where there is one. `--check` prints them; a render prints them (unless `-q`) and still writes the page.

The tooling crates under `scripts/` use `anyhow` at their entry points, with `.context(...)` saying what was being done. They write their output through `std::io::Write` on a locked handle and return a failed write like any other error (`say!` in `scripts/ci-pipeline/src/out.rs`). `md2html-version` does not exit the process either: `version_request` returns the version text, and each binary's `main` writes it and returns.

## Tests

Tests unwrap freely. Two lints still apply to them: `assertions_on_result_states` (`assert!(r.is_ok())` hides the error; `r.unwrap()` shows it) and `missing_assert_message` (every assertion says what it checks).
