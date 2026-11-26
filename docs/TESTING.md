<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# How md2html is tested

md2html promises three things, and the tests are arranged around them: the command line behaves as [docs/command-line.md](command-line.md) says, a Markdown text becomes the page it should (the CommonMark specification, then md2html's own rules), and no text makes a page that runs code or a run that hangs or crashes ([SECURITY.md](../SECURITY.md)). This page is for anyone who changes md2html or its tests: it names each kind of test, what it guards, where it runs, and how to add one. The lint rules for test code are those of all code ([policies/lint-posture.md](policies/lint-posture.md)); a test that fails gets a real fix, never a skip.

## The kinds of tests

| Kind | Where | What it guards |
| --- | --- | --- |
| Unit tests | `#[cfg(test)] mod tests` beside the code, or `<module>/tests.rs` | A module's own rules and edge cases: the scanner, escaping, SHA-256 and base64, the emphasis floors, a diagram's layout, the server's request reader |
| Whole pages | `src/tests/*.rs` | What a text becomes, read back from the page: `markdown.rs`, `reading.rs` and `spans.rs` (blocks, and the rules of blocks and of a line the specification's examples leave open), `glossary.rs`, `structure.rs` (the page around the article), `balanced.rs` (the author's raw HTML, however it fails to balance, leaves the page, the fragment and the book whole), `translations.rs`, `files.rs`, `escaping.rs` |
| CommonMark | `src/spec_tests.rs`, the examples in `crates/md2html/tests/commonmark/` (its `README.md` says how to list the failing ones) | The 652 examples of the specification, 0.31.2, compared as the specification's own test compares them: white space between tags aside, white space inside `<pre>` kept. The number passing is a floor that may rise, never fall |
| Hostile input | `src/tests/hostile.rs` | No script from the text; no way out of the page; the pathological shapes render whole and in linear time (within forty times plain text as long, measured beside it) |
| Generated documents | `src/tests/generated.rs` | 120 documents from a seeded generator, raw HTML opened and closed in any order among their pieces, with and without `--html`: every page whole, the same bytes twice, every element of it nested, no running code |
| Determinism | `src/tests/determinism.rs` | The same documents give the same bytes in another folder and another process (`one_run`, ignored, is the child it starts) |
| The command line's contract | `src/cli/contract_tests.rs` | Every option, setting, exit code and environment variable is on `docs/command-line.md` and in the code, and nothing else is |
| The heading anchors' contract | `src/tests/anchors.rs`, with `src/render/tests.rs` | A heading's anchor is GitHub's, and the same heading text gives the same id in every version: a table of heading texts and the ids GitHub's renderer gives them, repeated headings numbered as GitHub numbers them, and the page's own ids a heading never takes (`docs/policies/release_policy.md`) |
| The program as it is run | `crates/md2html/tests/command_line/` | The built binary in a folder of its own: where pages, books and PDFs go, a folder read as a handbook (its order, contents, glossary, images and warnings, served too), what each stream carries, each exit code from the situation that gives it, `--watch` after a save, `--pdf` with a stand-in Chrome (Unix) |
| Layers | `src/layer_tests.rs` | Uses go down `docs/ARCHITECTURE.md`'s layers and close no circle |
| Real connections | `src/serve/tests/tcp.rs` | The server over sockets: limits, refusals, slow and silent clients, a stop with connections open |
| What differs by system | `src/system/tests.rs` and the `cfg(unix)` / `cfg(windows)` twins in `src/tests.rs`, `cli/tests.rs`, `serve/tests.rs`, `export/images.rs`, `export/print.rs` | Separators, drives, links, case, line endings and names that are not Unicode, each held on the system it belongs to |
| Time | `src/tests/measure.rs` (ignored; `just measure`) | Where a render spends its time; a measurement to compare with main, not a gate |
| Doc tests | the tooling crates | Their examples compile and run; md2html is a binary and has none |
| Pages in a browser | `tests/browser/` (`audit.js`, `interact.js`, `live.js`, `nesting.js`, `handbook.js`) | The rendered pages in Chrome: WCAG 2.2 AA in both themes and at phone width, every control clicked, live reload keeping the reader's place, the article as Chrome reads it (the page's scripts off) nested as md2html wrote it (`balance.md`, rendered by default and with `--html`, puts the author's tags where the browser would end md2html's paragraph, list item, cell, link or heading), and the documentation as a handbook from `file://` (every page marks itself in the contents, next walks the contents first to last, an entry opens its page, the page's own outline still follows the reader) |
| Books | `scripts/ci/epubcheck_books.sh` (`just epubcheck`) | The 19 EPUBs of the repository's documents, and the book of `tests/browser/balance.md` with `--html` (md2html's blocks beside the author's open elements), pass epubcheck with no message |
| The repository's own pages | `md2html --check README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md` | Nothing to fix, and every translation up to date with its original |
| The differential test | `scripts/differential/` (`just differential [base]`) | 8,300 Markdown files rendered six ways by the build of `base` (origin/main unless named) and by this tree's, compared byte for byte; a change not meant to change output shows no difference |
| Generated documents, many | the same corpus (`just generated-documents`) | The 8,300 files rendered six ways by one build: none may panic, end by a signal or run past three minutes |
| The release's own scripts | `scripts/release/test_sign_macos.sh`, `scripts/release/test_winget_manifests.sh`, `scripts/release/test_release_notes.sh`, `scripts/release/test_install.sh` (gate `gate-tests`) | `just release-sign-macos` against stand-ins for `gh`, `codesign`, `xcrun`, `security`, `spctl`, `ditto` and `gopass`: every refusal before anything is signed, a refused notarization and a failed publish finished by a second run, exactly the six macOS assets and lists replaced; `pack_zip.sh` and `checksums.sh` giving the same bytes for the same files; the winget manifests of the first submission filled from a release, and their refusals; a release's notes, downloads first; the installer `install.sh` on each system under `/bin/sh` and dash, its refusals leaving nothing installed |
| The translation review tools | `scripts/i18n/test_i18n.sh` (gate `gate-tests`) | `words.py` reads every language's words with English's paths and writes a string back with its `{slots}` kept (a sentence that lost one refused); `review_site.py` writes a review per language asking who the reviewer is first, then every string, the seven terms (our guess marked, the least sure first) and every chapter, each a question of its own with its state and its Send; `review_answers.py` takes each reviewer's newest answer, leaves a withdrawn one out, and never prints how to reach a reviewer |
| Mutation testing | `cargo mutants` over `scripts/ci/mutants-scope.txt` (`just tier2-local`) | Every change to the logic of the listed modules makes a test fail |
| Miri, cargo-careful | `tier-2.yml` | The standard library's own checks under the parsers' tests |

## Where each runs

| When | What runs |
| --- | --- |
| Commit (`lint-fast`) | Format (Rust and TOML), file size, spelling, licences, the forge stamp, the audit store's format, clippy over production and test code |
| Push (`lint-pre-push`) | All of the above, then every test of the workspace (`nextest`, profile `pre-push-smoke`), doc tests, rustdoc, the drift gates, the documents' references (`doc-references`), the version in its three places (`versions`), md2html's tests on its minimum Rust (`msrv`), clippy for Windows, Linux and Intel Mac, the pushed commits' subjects and signatures; the supply chain (`deny`, `vet`) when a manifest, `Cargo.lock` or `supply-chain/` changed |
| Pull request, `PR Fast CI` | The gates of `scripts/ci/gates.toml`; the tests on Ubuntu with `nextest --profile ci` |
| Pull request and main, `ci.yml` | `cargo test --release` on Ubuntu, macOS and Windows; `--check` of the repository's pages; md2html's tests and release build on its minimum Rust (`msrv`); the browser suite; epubcheck; the supply chain |
| Weekly, `tier-2.yml` | Coverage, Miri, cargo-careful, cargo-udeps, the feature matrix, mutation testing, the generated documents |
| By hand, before a pull request that touches rendering | `just differential`; the browser suite (`node audit.js`, `node interact.js`, `node live.js`, `node nesting.js`, `node handbook.js` in `tests/browser`) when the output is meant to change |

The tests that need a file whose name is not Unicode on disk (`names_that_are_not_unicode_are_taken_as_they_are`) run on Linux and Windows: macOS's file system refuses such names. CI's Ubuntu and Windows test jobs run them; the other tests of such names build them in memory and run everywhere. The stand-in Chrome of `a_pdf_is_what_chrome_printed` is a shell script, so that test runs on Linux and macOS.

## Behaviour, not implementation

A test says what a person or a script sees: the page a text becomes, the file written, the exit code, the line on standard error. It is written against the narrowest public surface that shows the behaviour: a whole page or `--fragment` (`crate::tests::page`, `fragment`) for a rule of Markdown; the built program for the command line; a module's own functions only for a rule that lives there and nowhere else (the emphasis floors, the request reader). A test that would break when the code is rearranged without changing what anyone sees tests the arrangement, not the behaviour.

Each test's doc comment says what it holds and, under `# Panics`, what failing means. Its name is the rule in words (`a_bare_link_ends_before_the_sentence_does`), so a failure reads as the rule broken.

## Flaky tests

A flaky test is a test that fails, not a test to re-run. The `ci` profile of nextest retries a failed test three times and marks one that then passes as flaky, and `flaky-result = "fail"` fails the job: a flake cannot pass unnoticed. On 2026-11-24 the Actions history (744 runs since 2026-11-22) held no re-run and no flaky result; every failed test was a fault, fixed by a commit (the two Windows failures of the server's tests, in Phases 10 and 11).

What keeps a test from flaking:

- A folder of its own, named for the test and the process (`Scratch`, `Folder`), removed when it ends; never a fixed path, never the working directory.
- No sleep that hopes: a wait polls for the condition with a deadline (`wait_for` in `crates/md2html/tests/command_line/lasting.rs`), and the deadline is generous (20 seconds) because it is reached only by a failure.
- Time is compared, not clocked: a hostile shape must render within forty times the time of plain text as long, rendered just before on the same machine. A fixed limit of five seconds did fail once, under coverage on a machine busy with other work, for a 4 MB shape that is linear (2026-11-24); a machine slowed down slows both renders alike.
- A fixed date (`SOURCE_DATE_EPOCH`), no shared glossary, no Chrome unless the test brings one: the environment of a test is the test's.
- A port is asked of the system (`127.0.0.1:0`), never assumed free.

## Coverage

`cargo llvm-cov --branch -p md2html` measures it (`llvm-tools` comes with `just toolchain-ensure`, and the weekly job adds it; it uploads the result to Codecov). The tests that run the program count, as the program is built instrumented. On 2026-11-24, production code only (test modules left out): lines 92.0 %, branches 86.0 % (87.8 % and 82.5 % before the tests this page came with).

What is not covered, and why:

- `math/symbols.rs` (32 % of lines): tables of names and symbols, a `match` arm each. A test per entry would copy the table; the tests render a sample of each table, and a wrong symbol is a review question.
- `cli/live.rs`: the `--watch` loop's body runs under the test that kills it (a killed process writes no coverage), and `--serve`'s wiring is held by `tests/browser/live.js`, outside the measure; the server itself is tested in-process.
- `mermaid/` (89 %): rare shapes of diagram layout, each one a geometry case; the differential corpus renders 2,500 mutated diagrams.
- `export/print.rs`: the Chrome found in the usual places, and a Chrome that runs past 90 seconds.
- `serve/listener.rs`, `serve/http.rs` (82 %): the branches of I/O errors a real socket rarely gives.

## Mutation testing

`cargo mutants` changes the code one operator, condition or return value at a time and runs the tests on each change; a change no test notices (a missed mutant) is either a gap in the tests or code that does nothing. The weekly job mutates the files of `scripts/ci/mutants-scope.txt` and fails on any missed mutant; a timeout counts as caught. `just tier2-local` runs the same list locally.

| Module | Mutants | Missed | Time (6 jobs, local) | Weekly |
| --- | ---: | ---: | ---: | --- |
| `sanitize`, `text` | 381 | 0 | 4 min | since 2026-11-23 |
| `inline` | 365 | 0 | 6 min | since 2026-11-24 |
| `document` | 147 | 0 | 3 min | since 2026-11-24 |
| `block` | 333 | 0 | 6 min | since 2026-11-24 |
| `serve` | 180 | 50 | 4 min | next: most are I/O error kinds (a reader or writer that fails on demand would reach them) and the table of media types |
| `render` | 300 | 66 | 5 min | after `serve` |

To add a module: run `cargo mutants --package md2html -f '<module>.rs' -f '<module>/**/*.rs'`, read `mutants.out/missed.txt`, and for each missed mutant write the test that fails with it (the rule it breaks, at the level a reader sees it) or remove the code it shows to do nothing (a mutant no input can tell apart is dead code or a restated condition). When the module has none missed, add its globs to `scripts/ci/mutants-scope.txt` and its row above. The weekly job's limit is 120 minutes.

## Adding a test

| You changed | Add |
| --- | --- |
| A rule of Markdown | A case in `src/tests/markdown.rs` or `spans.rs`: the text, and the HTML of the page or fragment it must give; if the specification has an example for it, `spec_tests.rs` already holds it |
| What the page around the article holds | `src/tests/structure.rs` |
| An option, a setting, an exit code, an environment variable | Its row in `docs/command-line.md` (`contract_tests.rs` fails until it is there) and a test in `crates/md2html/tests/command_line/` that runs it |
| A document that names a file, a recipe or a gate | Nothing: the gate `doc-references` checks that each exists |
| The version, or the minimum Rust a document states | Nothing: the gate `versions` holds the three places and the statements to the manifests |
| A guard script under `scripts/` | Its `test_<name>.sh` beside it, planting each case it must refuse: the gate `gate-tests` runs every one |
| Something a hostile text could reach | `src/tests/hostile.rs`, with the attack string; a shape that could be slow gets a time bound there |
| What differs by operating system | A `cfg(unix)` / `cfg(windows)` pair, each holding its system's rule |
| The server | `src/serve/tests.rs` in-process, `serve/tests/tcp.rs` over a socket |
| A module in the mutation scope | Run `cargo mutants` on it; no new missed mutant |
| Anything that renders | Run `just differential`: no difference unless the change is meant to make one, and then each difference is read |
