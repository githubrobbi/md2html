# What makes a small Markdown-to-HTML CLI world-class (2025-2026): engineering, distribution, workflow, AI/agent era

Scope note: research run 2026-11-21. Budget-limited (about 15 fetches/searches). Claims marked "[canonical docs, not fetched this session]" come from well-known project docs that I did not re-read in this run. Treat them as pointers to verify, not as quotations. Local facts about md2html come from inspecting the working tree on 2026-11-21. That inspection found: no `.github/workflows` directory, a release binary of 468,336 bytes (`target/release/md2html`, built with lto=true, codegen-units=1, strip=true), and 1,755 lines across `src/*.rs`, not counting inline assets.

Std-only legend used below: **[RT-free]** means it can be done with no runtime dependency (std only, or dev-deps/CI tools only). **[RT-dep]** means it would normally pull a runtime crate, and the std-only alternative is noted.

## Testing: spec harnesses, golden files, fuzzing, HTML/a11y/visual checks, and how the leading parsers test

### Takeaway
The leading Rust Markdown projects all pair a spec-example harness with fuzzing. comrak and markdown-rs state their CommonMark/GFM pass counts publicly (652/652 and 670/670; "100%" plus 2300+ tests). mdBook goes further with a headless-browser GUI suite, and typst checks rendered output against small committed reference files with an `--update` flow. md2html can do all of this std-only by using dev-dependencies or CI-only tools.

### Cited Findings
- comrak's README badges claim 652/652 CommonMark spec examples (spec 0.31.2) and 670/670 GFM examples. The repo has a `fuzz` directory and a `spec_out.txt`. The project says CommonMark output is meant to re-parse to the same AST, which is a roundtrip property. — [comrak README](https://github.com/kivikakk/comrak)
- markdown-rs claims "2300+ tests": about 650 CommonMark tests plus "more than 1k extra tests" checked against CommonMark reference parsers, with more for GFM and extensions. It also claims 100% code coverage, 100% CommonMark, 100% GFM and 100% MDX. It ships cargo-fuzz (`markdown_libfuzz`) and honggfuzz (`markdown_honggfuzz`) targets, and it describes fuzzing as a check for cases that coverage might miss. — [markdown-rs README](https://github.com/wooorm/markdown-rs)
- markdown-rs warns that very large inputs or many unclosed constructs can cause slow processing or crashes. It recommends capping input size (it cites 500kb) and parsing in a separate thread. This is an honest pathological-input disclosure that md2html could copy. — [markdown-rs README](https://github.com/wooorm/markdown-rs)
- pulldown-cmark has a `fuzz/` directory and a separate `dos-fuzzer/` directory that targets denial-of-service (quadratic-time) inputs. It states the goal of "100% compliance" with CommonMark and has a `bench/` directory. CI runs from `.github/workflows/rust.yml`. — [pulldown-cmark README](https://github.com/pulldown-cmark/pulldown-cmark)
- pulldown-cmark keeps one spec-format file per extension in `pulldown-cmark/specs/`. The files are blockquotes_tags, cjk_friendly_emphasis, container_extensions, definition_lists, footnotes, heading_attrs, highlight, math, metadata_blocks, old_footnotes, **regression.txt**, strikethrough, super_sub, table and wikilinks. So every extension, and a running regression file, uses the same "markdown / expected HTML" example format as spec.txt. — [pulldown-cmark specs dir](https://github.com/pulldown-cmark/pulldown-cmark/tree/master/pulldown-cmark/specs)
- mdBook runs GUI tests from `tests/gui/runner.rs`. The runner builds sample books into a temp directory and drives headless Chrome with the `browser-ui-test` npm package. CI has a separate `gui` job (Node 24, `npm install`, eslint, then `cargo test --locked --test gui`), and the runner warns if the local browser-ui-test version differs from the pinned one. rustdoc uses the same tool for its GUI suite. — [mdBook 0.5.3 runner.rs](https://docs.rs/crate/mdbook/0.5.3/source/tests/gui/runner.rs); [mdBook 0.5.3 main.yml](https://docs.rs/crate/mdbook/0.5.3/source/.github/workflows/main.yml); [rustdoc GUI test suite](https://rustc-dev-guide.rust-lang.org/rustdoc-internals/rustdoc-gui-test-suite.html)
- typst's test suite compares each test's output with a reference in `tests/ref`. The runner detects visual output and then requires a reference image. Reference images are capped at 20 KiB (a `large` attribute overrides the cap, "very rarely"), `--update` regenerates and compresses references, and the docs say to prefer assertions over reference images where possible. — [typst tests README (mirror)](https://huggingface.co/spaces/ecyht2/typst-docs-rag/raw/main/sources/tests/README.md). Caveat: this is a mirror, so the current upstream layout is unconfirmed.
- Snapshot tooling [canonical docs, not fetched this session]: `insta` (`assert_snapshot!`, `cargo insta review`) is the de facto Rust snapshot crate and is a dev-dependency only. HTML validation is commonly done with the Nu Html Checker (`vnu.jar`) in CI. Accessibility checks commonly use axe-core (via `@axe-core/cli` or `@axe-core/playwright`). Visual regression uses Playwright's `expect(page).toHaveScreenshot()`. Property testing uses `proptest`/`quickcheck`. Fuzzing uses `cargo-fuzz` (libFuzzer) or `cargo-afl`. — [insta](https://insta.rs/); [Nu Html Checker](https://validator.github.io/validator/); [axe-core](https://github.com/dequelabs/axe-core); [Playwright visual comparisons](https://playwright.dev/docs/test-snapshots); [cargo-fuzz book](https://rust-fuzz.github.io/book/)

### Inferences
- **Spec harness [RT-free]:** vendor CommonMark `spec.txt` (0.31.2) plus md2html's own `tests/specs/*.txt` in the same format, following the pulldown-cmark layout. Parse the files with a roughly 40-line std-only test helper. md2html is deliberately not full CommonMark, so publish an honest pass count ("N/652 CommonMark examples") as a README badge or table, and keep a `known_failures.txt` allowlist so regressions fail CI but known gaps don't. This turns "checked by rendering real docs" into a measurable contract.
- **Golden files [RT-free]:** a `tests/golden/*.md` to `*.html` pair per feature, with an `UPDATE_GOLDEN=1 cargo test` path (typst's `--update` pattern). That works with std only. `insta` is an acceptable dev-dep alternative. Because md2html embeds large CSS/JS, snapshot only the `<main>` or body fragment, or strip the assets before comparing, so a CSS tweak doesn't churn every golden. typst's 20 KiB cap is the same idea.
- **Regression file [RT-free]:** every bug fix adds an example to a `regression.txt`, following pulldown-cmark.
- **Fuzzing [RT-free, dev/CI only]:** a `fuzz/` crate with one libFuzzer target (`render(&str)` must not panic, and must finish in time linear in the input size). Add a DoS-style target like pulldown-cmark's `dos-fuzzer` for nested emphasis, brackets and blockquotes. Run it in CI for a short time on a schedule. Fuzzing a binary-only crate needs a library target, which is another argument for a `lib.rs` (see the Library API section).
- **Property tests [RT-free with a dev-dep, or std-only with a tiny xorshift generator]:** check that output is always well-formed (balanced tags), that no raw `<script` appears from text input, and that rendering is idempotent and deterministic (same bytes twice).
- **HTML validity, a11y and visual checks [RT-free, CI only]:** render the docs corpus in CI, run `vnu --errors-only`, run axe-core through Playwright on each page in light and dark mode, and take Playwright screenshots at phone and desktop widths. This mirrors mdBook's GUI job and is how md2html's figure-enlarge, search and outline JS would get behavioural tests.
- md2html currently has about 10 unit tests and no CI workflow directory. The single biggest gap against the leaders is missing automation, not missing features.

### Gaps
- Could not confirm whether pulldown-cmark generates tests with `build.rs` (historically `tests/suite/*.rs` were generated from specs). The fetched directory listing did not say.
- typst's HTML-export test layout (`tests/ref/html`?) is unverified. Only mirrored README text was available.
- No primary source fetched on comrak's or mdBook's use of insta, or on how any of these projects run axe or vnu. I found no evidence that comrak, pulldown-cmark or markdown-rs run HTML validators or a11y tools in CI.

## CI and release: matrices, dist, Homebrew, binstall, crates.io, attestations/SBOM, semver, CHANGELOG, MSRV, lints, cargo-deny

### Takeaway
The 2025-2026 release baseline for a Rust CLI is `dist init`. That one command generates a tag-triggered `release.yml` that builds per-target archives, shell and PowerShell installers, a Homebrew formula pushed to a tap, and sha256 checksums. It can also turn on GitHub artifact attestations, `cargo auditable` and CycloneDX SBOMs. Comrak shows the distribution breadth a mature tool reaches, and an explicit MSRV policy like comrak's is now standard.

### Cited Findings
- `dist init` with GitHub CI enabled "will generate release.yml", which covers plan, build, host, publish and announce, and tagging a release triggers it. `release.yml` can also run the plan step, and optionally the build step, on PRs to catch release problems early. The listed installers are shell, powershell, npm, homebrew, msi and an updater. — [dist book intro](https://axodotdev.github.io/cargo-dist/book/introduction.html)
- dist config keys:
  - `github-attestations` (plus `-phase` and `-filters`) "will enable GitHub's experimental artifact attestation feature".
  - `cargo-auditable` builds with `cargo auditable build` to "embed metadata about your dependency tree into the built executables".
  - `cargo-cyclonedx` will "generate and upload a Software Bill Of Materials (SBOM)".
  - `installers`, plus `tap` ("a GitHub repository which dist should publish the Homebrew installer to").
  - `targets`.
  - `checksum` (`sha256`, `sha512` or `false`).
  - `publish-jobs` (`homebrew`, `npm`, or a custom job).
  - `unix-archive` (default `.tar.xz`).

  — [dist config reference](https://axodotdev.github.io/cargo-dist/book/reference/config.html)
- GitHub artifact attestations for a binary need the `id-token: write`, `contents: read` and `attestations: write` permissions, plus `uses: actions/attest@v4` with `subject-path`. Adding `sbom-path` makes it an SBOM attestation, and the action supports SPDX or CycloneDX predicates. Users verify with `gh attestation verify <binary> -R owner/repo`, adding `--predicate-type https://spdx.dev/Document/v2.3` for an SBOM. — [GitHub Docs: artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
- comrak's distribution includes crates.io (`cargo install comrak` and `cargo binstall comrak`), pacman, Homebrew, dnf, Nix, Scoop, prebuilt GitHub Release binaries and a webinstall.dev curl installer. — [comrak README](https://github.com/kivikakk/comrak)
- comrak's MSRV policy: Rust 1.89+. Patch releases never bump the MSRV. Minor releases may, but only to toolchains at least six months old. Major releases may bump it at any time. The Cargo features policy is covered by the stability guarantees. — [comrak README](https://github.com/kivikakk/comrak)
- pulldown-cmark states its MSRV in the README: "Rustc 1.71.1 or newer is required". — [pulldown-cmark README](https://github.com/pulldown-cmark/pulldown-cmark)
- [canonical docs, not fetched this session] Keep a Changelog defines the `## [Unreleased]` and Added/Changed/Deprecated/Removed/Fixed/Security format. `cargo-semver-checks` lints public API changes against the last published version. `cargo-deny` checks licenses, advisories, bans and sources. `cargo-binstall` resolves prebuilt binaries from GitHub Releases and can be steered with `[package.metadata.binstall]`. Clippy's `pedantic` group is opt-in through `[lints.clippy]` in Cargo.toml (stable since Rust 1.74). `cargo hack --rust-version` or a pinned MSRV CI job verifies the MSRV. — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks); [cargo-deny](https://github.com/EmbarkStudios/cargo-deny); [cargo-binstall](https://github.com/cargo-bins/cargo-binstall); [Cargo lints table](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)

### Inferences
- **CI matrix [RT-free]:** ubuntu, macos and windows on stable, plus one MSRV (1.85) job. Each runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (with `[lints.clippy] pedantic = "warn"` and selective allows), `cargo test`, and the docs-corpus render plus vnu and axe job. A std-only crate makes `cargo-deny` nearly trivial, but running it still documents the zero-dependency claim and catches accidental additions, so add a `bans` deny for any non-dev dependency.
- **Release [RT-free]:** `dist init` with `installers = ["shell", "powershell", "homebrew"]`, `tap = "githubrobbi/homebrew-tap"`, `github-attestations = true`, `cargo-cyclonedx = true` and `cargo-auditable = true`. With zero dependencies, the SBOM and auditable metadata are nearly empty, and that is itself a supply-chain selling point ("SBOM: 0 third-party components"). Note the attestation key is described as "experimental" in dist's docs.
- **crates.io:** flipping `publish = false` enables `cargo install md2html` and `cargo binstall md2html`. Check name availability first. A dist-built GitHub Release is what makes binstall fast.
- **Reproducible builds [RT-free]:** a std-only binary with `strip = true` and `codegen-units = 1` is a good candidate. Set `SOURCE_DATE_EPOCH`, use `--remap-path-prefix`, and add a CI job that builds twice and compares hashes. Make sure md2html never embeds a build timestamp, and that it emits deterministic output (no current-time stamps in generated HTML unless the user asks for them).
- **Semver and CHANGELOG:** md2html's commit messages already carry versions ("(0.3.3)"). A Keep a Changelog `CHANGELOG.md` is cheap, and dist can use it to fill release notes. If md2html gains a lib API, add `cargo-semver-checks` to CI. Before that, semver applies to CLI flags and output format, so document what counts as breaking: flag removals, and changes to element IDs or classes that users' CSS or scripts target.
- Publish an explicit MSRV policy in comrak's style. Example: "MSRV 1.85 (edition 2024 minimum); raised only in minor releases, to toolchains at least 6 months old."

### Gaps
- Could not confirm from the fetched pages whether dist is still actively maintained by axo in late 2026, or how dist interacts with binstall metadata. The dist book's binstall coverage was not on the fetched pages.
- The SLSA level reached by `actions/attest` was not stated on the fetched GitHub page.
- No primary source fetched for ripgrep's, bat's or zola's CI files. Their matrices and release processes are not documented here.

## CLI UX: stdin/stdout, batch, watch, open, config, exit codes, check/lint mode, diagnostics, completions, man page

### Takeaway
clig.dev gives the checklist. A file converter should accept `-` for stdin and stdout, keep primary output on stdout and messages on stderr, map exit codes to failure classes, lead `--help` with examples, honour `NO_COLOR`, and write atomically (temp file then rename). comrak shows the mature-converter defaults: stdin when no file is given, a config file, `-e` extension flags and `--inplace`. Every item below can be done std-only.

### Cited Findings
- clig.dev on input and output:
  - "If input or output is a file, support `-` to read from `stdin` or write to `stdout`."
  - "The primary output for your command should go to `stdout`," while "Log messages, errors, and so on should all be sent to `stderr`."
  - "If your command is expecting to have something piped to it and `stdin` is an interactive terminal, display help immediately and quit."

  — [clig.dev](https://clig.dev/)
- clig.dev on exit codes: "Return zero exit code on success, non-zero on failure", and "Map the non-zero exit codes to the most important failure modes." — [clig.dev](https://clig.dev/)
- clig.dev on help: "Display concise help text by default," "Lead with examples," and "Consider providing man pages," reachable through the tool itself. — [clig.dev](https://clig.dev/)
- clig.dev on errors: "Catch errors and rewrite them for humans", suggest the likely intended command, put the key failure last, and group repeated errors under one header. — [clig.dev](https://clig.dev/)
- clig.dev on machine output and quiet mode: "Display output as formatted JSON if `--json` is passed", "you can provide a `-q` option to suppress all non-essential output", and "If you change state, tell the user", for example by printing the output path. — [clig.dev](https://clig.dev/)
- clig.dev on colour: honour `NO_COLOR` when "set and it is not empty (regardless of its value)", disable colour when not on a TTY, and check stdout and stderr separately. — [clig.dev](https://clig.dev/)
- clig.dev on robustness: "operations should be idempotent where possible", "Make it crash-only" (write to a temp file and rename on success so partial outputs don't remain), and exit promptly on Ctrl-C. "Confirm before doing anything dangerous", for example overwriting, with `-f`/`--force` for scripts. "Never _require_ a prompt." Offer `-n, --dry-run`. — [clig.dev](https://clig.dev/)
- clig.dev on config: precedence runs flags > env vars > project config > user config > system config, and user config should follow XDG. — [clig.dev](https://clig.dev/)
- comrak's CLI reads a file or stdin when no file is passed and reads a config file by default (disabled with `--config-file none`). It outputs HTML, XML or CommonMark (`-t`), enables extensions with `-e` (comma list or repeated), and has `--gfm`, `--unsafe`, `--sourcepos`, `--syntax-highlighting` and `--inplace`. — [comrak README](https://github.com/kivikakk/comrak)
- clig.dev does not cover shell completions (the fetch found nothing on them). — [clig.dev](https://clig.dev/)

### Inferences (each [RT-free] unless noted)
- **`md2html -` and `-o -`:** read stdin and write stdout. This is the most agent-friendly addition, because an agent can pipe `… | md2html - -o report.html` with no temp file. When stdin is a TTY and `-` is not given, print short help (clig).
- **Multiple files, globs, and `--out-dir`:** render many pages per run. Group errors per file and return a nonzero exit code if any file fails.
- **`--watch` [RT-free]:** poll `std::fs::metadata(..).modified()` every 250-500 ms and re-render on change. For live reload, write a tiny std `TcpListener` HTTP server that serves the page plus a long-poll or SSE endpoint, and inject a reload snippet only in watch mode. Alternatively, skip the server and add `<meta http-equiv="refresh">` only in watch mode, which is std-only and zero-JS but crude. Keep the injected snippet out of normal output so output stays deterministic.
- **`--open`:** spawn `open` (macOS), `xdg-open` (Linux) or `cmd /c start` (Windows) with `std::process::Command`.
- **`--check` / `--lint`:** parse without writing, and report constructs md2html does not support, such as raw HTML blocks, reference-style link definitions with no uses, unclosed fences, heading-level jumps, broken intra-document anchors and missing image files. Print `file:line:col: warning: …` to stderr and exit 1 when issues are found. This is high value for agents: an agent can run `md2html --check` after writing a document and fix issues before publishing. Add `--json` diagnostics so agents parse the results reliably (clig).
- **Diagnostics without miette:** a source-line excerpt with a caret underline and a `help:` line, coloured only when stderr is a TTY and `NO_COLOR` is unset. About 50 lines of std. Note that std has `std::io::IsTerminal`, stable since Rust 1.70.
- **Exit codes:** 0 ok, 1 lint findings or render warnings promoted by `--strict`, 2 usage error, 3 input unreadable, 4 output unwritable. Document them in `--help` and the README.
- **Atomic writes:** write `out.html.tmp`, then `rename`, following clig's "crash-only" advice. This also stops a browser that is watching the file from seeing half a page.
- **Config file:** an optional `md2html.toml` in the project, or `$XDG_CONFIG_HOME/md2html/config`. TOML parsing is [RT-dep] (`toml` crate). The std-only alternative is a trivial `key = value` format, or YAML front matter in the document itself, which md2html may already read for the title. Front matter per document is more agent-friendly than a global config.
- **Shell completions and man page:** usually generated by `clap_complete` and `clap_mangen` [RT-dep / build-dep]. The std-only alternative is to hand-write `completions/md2html.{bash,zsh,fish}` and a `md2html.1` in roff (or render the man page from a Markdown source at release time), and ship them in the dist archive. With a small flag set this is a few dozen lines each.

### Gaps
- No primary source fetched for ripgrep's or bat's specific exit-code conventions or completion-generation approach.
- I found no published guidance specific to "lint mode for Markdown renderers". markdownlint (Node) is the closest analogue, not researched here.

## Library API, WASM playground, editor integrations, templates/theming

### Takeaway
Every leading Markdown crate is a library first and a CLI second. comrak ships WASM bindings for JS/TS on npm and JSR plus Ruby, Elixir and Python bindings, and markdown-rs is `#![no_std]` + `alloc`. A `lib.rs` exposing `render(&str, &Options) -> String` would let md2html be fuzzed, embedded, compiled to wasm32 for a browser playground, and wrapped as an MCP tool, all without runtime dependencies.

### Cited Findings
- comrak lists WebAssembly-based JavaScript/TypeScript bindings on npm and JSR, plus Ruby, Elixir and Python bindings. A third-party Python binding claims to be 15-60x faster than pure-Python parsers. — [comrak README](https://github.com/kivikakk/comrak)
- markdown-rs is "a state machine implemented as `#![no_std]` + `alloc`", is "100% safe Rust", and is "100% safe HTML by default". Dangerous HTML and protocols are encoded or dropped unless `allow_dangerous_html` or `allow_dangerous_protocol` is set. — [markdown-rs README](https://github.com/wooorm/markdown-rs)
- pulldown-cmark is pure Rust with no unsafe code, except in the opt-in SIMD feature for x64. — [pulldown-cmark README](https://github.com/pulldown-cmark/pulldown-cmark)
- [canonical docs, not fetched this session] The Rust API Guidelines checklist covers naming (C-CASE), common trait impls (C-COMMON-TRAITS), builder or options structs with `#[non_exhaustive]` for future-proofing (C-STRUCT-PRIVATE), documented errors and panics (C-FAILURE), and examples in docs (C-EXAMPLE). — [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html)

### Inferences
- **Lib plus bin split [RT-free]:** `src/lib.rs` with `pub fn render(markdown: &str, opts: &Options) -> Page` (or `-> String`), `#[non_exhaustive] pub struct Options`, and `pub fn check(markdown: &str) -> Vec<Diagnostic>`. Keep `main.rs` as a thin argument parser. This enables fuzzing, docs.rs, and reuse by other Rust tools and agent harnesses.
- **WASM playground [RT-free for the core]:** std-only code compiles to `wasm32-unknown-unknown` unless it touches fs, process or time. Isolate I/O in `main.rs`. A raw `extern "C"` export plus a handwritten 30-line JS shim avoids `wasm-bindgen`, which is a build-time tool, not a runtime dependency of the native binary. Host it on GitHub Pages as "paste Markdown, see the md2html page, download the HTML". This is the "playground link" top projects put in their README.
- **Safety defaults:** following markdown-rs, document md2html's raw-HTML policy explicitly (escape, pass through, or flag), and offer a `--safe` mode. This matters when agents render untrusted text such as web-scraped content or emails.
- **Templates/theming [RT-free]:** a pandoc-style `--template file.html` with `{{title}}`, `{{toc}}`, `{{body}}`, `{{css}}` and `{{source}}` placeholders, filled by plain string replacement. Add `--css extra.css` to append a stylesheet, and CSS custom properties on `:root` as the documented theming surface. Documenting the stable class and ID names is part of semver (see the CI section).
- **Editor integration:** a VS Code "md2html preview" extension can shell out to the binary or use the WASM build. This is lower priority than stdin/stdout, which every editor's "run external command on buffer" feature can already use.

### Gaps
- Did not fetch pandoc's template docs or mdBook's theme docs, so the exact placeholder conventions used by those tools are not cited.
- No data found on how often small Markdown CLIs publish VS Code extensions, or on whether those extensions get used.

## AI/agent era: llms.txt, embedded Markdown source, machine-readable metadata, share tools, MCP, which features matter for LLM output

### Takeaway
Agent-friendliness for a document renderer has settled into four conventions:
- a Markdown twin of every HTML page (`page.html.md`, or `rel="alternate" type="text/markdown"`)
- a "Copy as Markdown" / "View as Markdown" button
- llms.txt for multi-page sets
- MCP or HTTP endpoints so agents can publish without a human

A wave of 2025-2026 hosted tools (md.page, RenderMark, mdview.io, ShareMyPage) exists only to turn agent Markdown into shareable pages, and Mermaid support is their most common headline feature. md2html can be the local, offline, single-file, zero-dependency version of that category.

### Cited Findings
- **llms.txt.** Jeremy Howard proposed it on 2024-09-03, and the current version (v2) was last modified 2026-08-10. It is "A proposal to standardise on using an `/llms.txt` file to provide information to help agents use a website."
  - The only required section is an H1. Then come a blockquote summary, optional paragraphs (no headings), H2 sections of Markdown link lists with optional `: notes`, and an "Optional" H2 for skippable links.
  - The file can live at any subpath and covers URLs beneath it.

  — [llmstxt.org](https://llmstxt.org/)
- **Markdown twins.** llms.txt v2 says pages should offer a clean Markdown version at the same URL, "either with `.md` appended (`page.html.md`) or with the extension replaced by `.md` (`page.md`)".
  - Clients discover the twin through `rel="alternate" type="text/markdown"` links or an HTTP `Link:` header.
  - `rel="describedby"` points to the covering llms.txt.

  — [llmstxt.org](https://llmstxt.org/)
- **llms.txt adoption.** Chrome Lighthouse audits for an llms.txt file in its agentic-browsing checks. OpenAI, Anthropic and Gemini publish llms.txt files for their developer docs. Mintlify, GitBook, Yoast, AIOSEO and Wix generate the file. nbdev creates Markdown versions of all pages by default. Plugins exist for VitePress and Docusaurus, and there is an MCP server (server-llm-txt). — [llmstxt.org](https://llmstxt.org/). Caveat: these adoption claims come from the proposal's own site.
- **"Copy as Markdown" / "View as Markdown" is now a widespread docs-site convention:**
  - Fastly puts both buttons under each title.
  - Langfuse shipped "Copy as Markdown" in April 2025 and `.md` endpoints in August 2025.
  - Vercel has "Copy page", with "View as Markdown" opening the `.md` URL.
  - Mintlify appends `.md` and supports Cmd/Ctrl+C to copy the page as Markdown.
  - Fern has "View as Markdown" on by default.
  - Appwrite, Cloudflare, Checkly, Grafana and Microlink use `.md` suffixes, and several also negotiate `Accept: text/markdown`.

  — [Fastly](https://www.fastly.com/documentation/reference/markdown/); [Langfuse 2025-04-17](https://langfuse.com/changelog/2025-04-17-copy-as-markdown.md); [Langfuse 2025-08-07](https://langfuse.com/changelog/2025-08-07-markdown-endpoints-and-copy-button.md); [Vercel](https://vercel.com/docs/agent-resources/markdown-access); [Mintlify](https://www.mintlify.com/docs/ai/markdown-export); [Fern](https://buildwithfern.com/learn/docs/developer-tools/view-markdown); [Cloudflare](https://developers.cloudflare.com/fundamentals/reference/markdown-for-agents/index.md); [Grafana 2026-05-15](https://grafana.com/whats-new/2026-05-15-grafana-documentation-now-ships-as-markdown/)
- There is no single canonical spec for the copy-button or `.md`-endpoint convention. Naming varies by vendor. — [search synthesis over the sources above]
- **md.page.** Agents publish by `POST https://md.page/api/publish` (JSON in, `{url, expires_at}` out), through a remote MCP endpoint at `https://md.page/mcp` that also works as a Claude.ai custom connector, through a local stdio MCP (`npx -y mdpage-mcp`), or through a Claude Code skill. Anonymous pages expire in 24 hours. Mermaid is "Built-in support", and Open Graph images are generated automatically. — [md.page](https://md.page/)
- **Other share tools:**
  - RenderMark offers an MCP server that lets an agent preview, export, publish or share a document, and added PDF, DOCX and HTML exports in June 2026.
  - mdview.io offers share links and API tokens for agent or CI publishing.
  - ShareMyPage shares AI-made HTML and Markdown pages behind company sign-in.
  - HTML Anything is open source and turns Markdown and data into single-file HTML or PNG using locally logged-in coding agents.
  - A "markdown_to_html" MCP connector tool escapes raw HTML in its input.
  - Svelte-markdown documents patterns for rendering agent output.

  — [RenderMark](https://rendermark.app/); [RenderMark June 2026](https://rendermark.app/blog/new-agent-features-june-2026); [mdview.io](https://mdview.io/s/ai-markdown-viewer); [ShareMyPage](https://sharemypage.app/); [HTML Anything](https://silenceper.com/en/article/2026-05-23-html-anything-agentic-html-editor/); [Glama markdown_to_html](https://glama.ai/mcp/connectors/io.github.Nutshell2000/agent-data-tools-utilities/tools/markdown_to_html); [svelte-markdown agent output](https://markdown.svelte.page/docs/advanced/agent-output). Caveat: these are mostly vendor marketing pages.
- A page titled "HTML vs Markdown for AI Agents: Claude Code Team Switch Explained" appeared in search, but it returned HTTP 410 Gone. Its claim is unverified and should not be cited. — [anycap (410)](https://anycap.ai/page/en-US/ai/html-vs-markdown-ai-agent-output)

### Inferences
- **Embed the source [RT-free]:** put the original Markdown in the page as `<script type="text/markdown" id="md-source">…</script>`, escaping `</script` as `<\/script`. Add a "Copy as Markdown" button (Clipboard API) and a "Download .md" link (Blob URL). The page then round-trips: an agent receiving the HTML can recover the exact source, matching the docs-site convention above without a server. Make it opt-out (`--no-source`) for size or privacy.
- **Markdown twin [RT-free]:** an optional `--emit-md-twin` writes `page.html.md` next to `page.html`, plus `<link rel="alternate" type="text/markdown" href="page.html.md">` in `<head>` (the llms.txt v2 convention). In directory or batch mode, generate an `llms.txt` index from H1s and first paragraphs, with an `## Optional` section.
- **Machine-readable metadata [RT-free]:** `<meta name="generator" content="md2html 0.3.3">`, title/description/date/author from front matter, `<meta property="og:*">`, and a `<script type="application/ld+json">` with `@type: Article` or `TechArticle`, `headline`, `dateModified` and `wordCount`. The JSON is trivial to emit by hand with careful escaping. Also embed a source hash (`<meta name="md2html:source-sha256">`) so agents can tell whether a page is stale against its .md. SHA-256 is about 100 lines of std code, or use a cheaper std-only FNV/xxhash-style fingerprint if it is labelled non-cryptographic.
- **Which features matter for LLM-written docs:** what agents actually emit is GFM tables, fenced code with language tags, task lists, GitHub-style alerts (`> [!NOTE]`), footnotes, Mermaid fences, and `$…$` / `$$…$$` math. Mermaid is the headline feature for share tools (md.page). Mermaid and KaTeX are large JS libraries, so std-only, self-contained, offline output conflicts with them. The options are:
  1. keep the fence as styled source and lazy-load mermaid from a CDN only when online (`--mermaid=cdn`)
  2. inline a vendored minified mermaid only when the document contains a mermaid fence, which keeps the size impact per document
  3. render a small subset (flowchart) natively to SVG in Rust, which is a big effort

  Math has the same trade-off, with MathML as a zero-JS option (native in all major browsers since Chrome 109).
- **Robustness to LLM output:** agent Markdown often has unclosed fences, mixed list markers and HTML fragments. A `--check` mode, plus graceful rendering of an unterminated fence (close it at end of file with a visible warning), are differentiators.
- **MCP server:** md.page and RenderMark show that agents publish through MCP. A `md2html mcp` subcommand speaking JSON-RPC over stdio, with tools `render(markdown) -> html_path` and `check(markdown) -> diagnostics`, can be written std-only (a line-delimited JSON reader and writer; a hand-rolled minimal JSON parser is about 300 lines). Offline, local and private, it is the counterpart to the hosted share tools. Be careful: a hand-rolled JSON parser is a fuzzing target too.
- **Positioning:** "md.page/RenderMark, but local, offline, one file, zero dependencies, no expiry, no upload" is a crisp niche statement for the README.

### Gaps
- Could not verify any primary statement from Anthropic, OpenAI or Notion about how Claude artifacts, ChatGPT canvas or Notion AI render Markdown, or which features they support (the anycap page was gone, and no first-party docs were fetched). HackMD, Obsidian Publish and Gist renderers were not researched in this run.
- No quantitative data found on which Markdown constructs LLMs emit most often. The feature list above is an inference, not a measured distribution.
- The MCP specification itself was not fetched, so the exact message shapes for a stdio server should be checked at modelcontextprotocol.io.
- llms-full.txt is not mentioned on the current llmstxt.org page. Its status as a convention is unverified here.

## Benchmarks, performance claims, and binary size

### Takeaway
Performance numbers in this space are mostly self-reported by competing projects and not comparable across hardware. Roughly, pulldown-cmark (pull parser, no AST) runs about 3.7x faster than comrak (AST) in one competitor's measurement, and comrak openly says it "is not and will not be the fastest". md2html's 468 KB stripped std-only binary is a concrete, defensible claim. It is better to publish a reproducible hyperfine script than headline MB/s figures.

### Cited Findings
- comrak's Makefile has `make bench-comrak`, which runs hyperfine on the release binary, and `make bench-all`, which compares against cmark-gfm, pulldown-cmark and markdown-it.rs. The author says comrak "is not and will not be the fastest" in stdin-to-stdout tests because some alternatives don't build an AST. — [comrak README](https://github.com/kivikakk/comrak)
- ferromark (0.1.4, a competitor) self-reports comrak at about 67.9 MiB/s and pulldown-cmark at about 254.5 MiB/s on a 5 KB input, with pulldown-cmark roughly at parity with md4c. — [ferromark README](https://docs.rs/crate/ferromark/0.1.4/source/README.md). Caveat: this is a self-published comparison.
- rushdown's README reports comrak at about 4.01 ms, pulldown-cmark at about 5.59 ms and cmark at about 3.49 ms on its input. This contradicts ferromark's ordering of comrak and pulldown-cmark, likely because of different inputs, versions or options. — [rushdown README](https://docs.rs/crate/rushdown/0.9.1/source/README.md); contradicted by [ferromark README](https://docs.rs/crate/ferromark/0.1.4/source/README.md)
- A 2016 forum post by comrak's author reported comrak at about 1.9x cmark's runtime. pulldown-cmark's speed is attributed to returning offsets into the source rather than copying strings. — [users.rust-lang.org](https://users.rust-lang.org/t/release-comrak-commonmark-gfm-compatible-markdown-parser/10340)
- pulldown-cmark offers an opt-in SIMD feature on x64, its only unsafe code. — [pulldown-cmark README](https://github.com/pulldown-cmark/pulldown-cmark)
- One comrak fork notes that cmark-gfm no longer appears to be actively maintained. — [philomena-dev/comrak](https://github.com/philomena-dev/comrak)
- Local measurement on 2026-11-21: md2html 0.3.3's release binary is 468,336 bytes (about 457 KiB) on macOS arm64, built with `lto = true`, `codegen-units = 1` and `strip = true`. — [local: /Users/rnio/Private/Github/md2html/target/release/md2html, Cargo.toml lines 25-28]

### Inferences
- **Benchmark harness [RT-free]:** add a `bench/` directory with a corpus: the CommonMark spec as one large file, the md2html README, and a synthetic 10 MB document. Add a `bench.sh` that runs `hyperfine` against md2html, comrak, pulldown-cmark's CLI and pandoc, using comrak's Makefile pattern. Publish the results table with the hardware and versions listed, and state that output includes embedded CSS/JS, so it measures end-to-end page generation, not only parsing. A std-only `cargo bench` alternative is a `#[test] #[ignore]` timing loop with `std::time::Instant`. `criterion` is a dev-dep option.
- **Pathological-input guarantee:** state and test linear-time behaviour on nested-bracket and emphasis inputs (pulldown-cmark's dos-fuzzer, markdown-rs's 500kb guidance). That claim is worth more than raw MB/s.
- **Binary-size table:** measure md2html beside `comrak`, `pandoc` and `mdbook` on the same machine for the README. pandoc is a Haskell binary that is widely known to be tens of MB, but I did not measure it in this run, so measure it before publishing. Also show "runtime dependencies: 0" from `cargo tree -e normal`.

### Gaps
- No independent, apples-to-apples Markdown parser benchmark was found. All numbers are self-reported by projects.
- Binary sizes of comrak, pandoc and mdBook were not measured in this run (the tools are not installed locally).

## README and docs practices of top CLI projects

### Takeaway
Top projects put verifiable claims above the fold: spec pass counts as badges (comrak 652/652, 670/670), install lines for every channel (comrak lists eight channels), an explicit MSRV and stability policy, and honest performance positioning. Docs sites in the agent era add Markdown twins and copy buttons. md2html's README should add an install matrix, a screenshot or GIF of a rendered page, a live sample page and playground link, a comparison table, and a "for agents" section.

### Cited Findings
- comrak's README leads with spec-compliance badges, then lists installation channels (crates.io, binstall, pacman, Homebrew, dnf, Nix, Scoop, GitHub Releases, webinstall.dev). It documents CLI flags, bindings in other languages, an MSRV and stability policy, and benchmark commands. — [comrak README](https://github.com/kivikakk/comrak)
- markdown-rs's README states its quality claims as a feature list ("100% safe Rust", "100% CommonMark", "2300+ tests", "100% coverage") and includes a security section that warns about specific options and pathological inputs. — [markdown-rs README](https://github.com/wooorm/markdown-rs)
- pulldown-cmark's README states its design goals (fast, safe, versatile, correct), the MSRV, and its unsafe-code policy. — [pulldown-cmark README](https://github.com/pulldown-cmark/pulldown-cmark)
- mdBook documents CI usage for books in its user guide ("Running mdbook in Continuous Integration"). — [mdBook CI guide](https://rust-lang.github.io/mdBook/continuous-integration.html)
- clig.dev says to "Provide web-based documentation" that users can search and link to, and to lead help text with examples. — [clig.dev](https://clig.dev/)

### Inferences
- **README skeleton for md2html:**
  1. A one-line pitch, plus a screenshot of a rendered page in light and dark mode. A GIF of figure-enlarge or search is optional.
  2. A link to a live sample page (the README itself rendered by md2html, hosted on GitHub Pages) and the WASM playground.
  3. An install matrix covering brew, `cargo binstall`, `cargo install`, the shell installer and Releases.
  4. A 30-second usage section that leads with examples: file, stdin pipe, batch, watch.
  5. A "for AI agents" section: the pipe usage, `--check --json`, embedded source and Copy-as-Markdown, MCP mode, and a sample instruction to put in an agent's CLAUDE.md or AGENTS.md.
  6. A supported-syntax table with a spec pass count.
  7. A comparison table against pandoc, comrak, mdBook, md.page and grip, covering self-contained output, offline use, runtime dependencies, binary size, single page vs site, Mermaid, math and agent features.
  8. Stability, MSRV and security (raw-HTML) policy.
  9. Benchmarks with a reproduce script.
  10. CHANGELOG and contributing links.
- **Dogfooding:** render the README, CHANGELOG and docs with md2html in CI, publish them to GitHub Pages, and add the Markdown twin and llms.txt. The project's own site then demonstrates every agent-era convention.

### Gaps
- No primary sources fetched for ripgrep's, bat's, zola's or typst's READMEs or CONTRIBUTING files. Their specific README patterns (ripgrep's benchmark-heavy README, bat's demo GIF) are not cited here and should be verified before they are used as examples.
- No data found on whether demo GIFs or comparison tables measurably affect adoption.
