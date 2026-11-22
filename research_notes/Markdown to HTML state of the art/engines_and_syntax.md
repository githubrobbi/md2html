# Markdown engines and syntax extensions: state of the art (as of Nov 2026)

Scope note: research done 2026-11-21 with web search/fetch of primary pages (READMEs, docs.rs, GitHub docs, typst.app, caniuse). Items marked "(background, unverified)" are from prior knowledge and were not re-checked in this pass. The local docenta corpus held no prior md2html research on this topic (search for "pulldown-cmark commonmark" returned only unrelated hits).

## 1. Leading engines: status, versions, spec compliance, speed, safety, extensions

### Takeaway
The Rust field is mature and converged on CommonMark 0.31.2: comrak (0.59.0, Oct 2026) passes 652/652 CommonMark and 670/670 GFM spec tests and ships the broadest extension set; pulldown-cmark (0.13.x, 2026) is the fastest pure-Rust, no-unsafe pull parser with 15 extension flags including GitHub alerts, math, definition lists and wikilinks; markdown-rs claims 100% CommonMark plus 1k+ extra tests and fuzzing. Typst's HTML export is still experimental (0.14, Oct 2025). None are std-only/zero-dependency in the md2html sense except that pulldown-cmark's core has very few deps.

### Cited Findings

**pulldown-cmark (Rust)**
- Latest docs.rs version seen: 0.13.6 (Options page) — [docs.rs Options](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html). A package index listed 0.13.2 (21 Mar 2026) as latest at its crawl time, so several 0.13.x patch releases shipped in 2026 — [ecosyste.ms](https://packages.ecosyste.ms/api/v1/registries/crates.io/packages/pulldown-cmark). A March 2026 blog post says 0.13.1 supports CommonMark 0.31 — [dasroot.net](https://dasroot.net/posts/2026/03/building-markdown-parser-rust-pulldown-cmark/).
- Goals stated in README: "Correct; the goal is 100% compliance with the CommonMark spec"; "Fast; a bare minimum of allocation and copying"; "Safe; written in pure Rust with no unsafe blocks (except in the opt-in SIMD feature)." SIMD scanners for x64 since 0.5. `std` feature can be disabled for `no_std` (with `hashbrown`). MSRV 1.71.1 — [README](https://cdn.jsdelivr.net/gh/pulldown-cmark/pulldown-cmark@main/README.md).
- Extension flags in 0.13.6: ENABLE_TABLES, ENABLE_FOOTNOTES, ENABLE_STRIKETHROUGH, ENABLE_TASKLISTS, ENABLE_SMART_PUNCTUATION, ENABLE_HEADING_ATTRIBUTES (id, classes, custom attrs), ENABLE_YAML_STYLE_METADATA_BLOCKS (`---`…`---`/`...`), ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS (`+++` TOML-style), ENABLE_OLD_FOOTNOTES, ENABLE_MATH (emits InlineMath/DisplayMath events with raw TeX — no rendering), ENABLE_GFM (currently = GitHub blockquote alert tags NOTE/TIP/WARNING…), ENABLE_DEFINITION_LIST (commonmark-hs-extensions compatible), ENABLE_SUPERSCRIPT, ENABLE_SUBSCRIPT, ENABLE_WIKILINKS (Obsidian-style) — [docs.rs Options](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html). README also mentions `==highlight==` → `<mark>` — [README](https://cdn.jsdelivr.net/gh/pulldown-cmark/pulldown-cmark@main/README.md).
- Speed (competitor-authored benchmark, 50 KB input): pulldown-cmark ~275 MiB/s, md4c (C) ~253 MiB/s, comrak ~72 MiB/s — [ferromark README](https://docs.rs/crate/ferromark/0.1.4/source/README.md). Treat as directional; it is a rival project's own measurement.
- A fork "satteri-pulldown-cmark" exists (0.5.2, Jun 2026) — not upstream — [docs.rs](https://docs.rs/crate/satteri-pulldown-cmark/0.2.5).

**comrak (Rust, cmark-gfm port)**
- 0.59.0 released 2026-10-26; badges "CommonMark: 652/652", "GFM: 670/670"; "Compliant with CommonMark 0.31.2 by default"; library MSRV Rust 1.89+ — [docs.rs comrak 0.59.0](https://docs.rs/crate/comrak/0.59.0).
- Extensions (all off by default, enabled per parse): GFM tables, task lists, strikethrough, autolinks; plus superscript, header IDs, footnotes, inline footnotes, description lists, front matter, multi-line blockquotes, math, emoji shortcodes, wikilinks, underline, spoiler, "greentext", alerts, CJK-friendly emphasis. The GFM tagfilter ("Disallowed Raw HTML") extension was deprecated in 0.55.0 and removed in 0.56.0 — [docs.rs comrak 0.59.0](https://docs.rs/crate/comrak/0.59.0).
- Safety default: "As with cmark and cmark-gfm, Comrak will scrub raw HTML and potentially dangerous links"; `unsafe` option to allow; recommends pairing with a sanitizer such as `ammonia` — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0).
- Highlighting via a `SyntaxHighlighterAdapter` trait, with a bundled syntect adapter — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0).
- Self-assessed speed: "Comrak is not and will not be the fastest" (builds an AST) — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0). Older forum post: comrak reached 1.9x cmark's runtime (from 4.5x) — [users.rust-lang.org (older)](https://users.rust-lang.org/t/release-comrak-commonmark-gfm-compatible-markdown-parser/10340). Continuous benchmarks on CodSpeed — [codspeed](https://app.codspeed.io/kivikakk/comrak).
- Dependencies are substantial (arbitrary, bon, caseless, emojis, finl_unicode, memchr, phf, smallvec, typed-arena, syntect, clap, …; several optional) — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0).

**markdown-rs (wooorm, Rust port of micromark)**
- Claims "100% to CommonMark"; "tested with the ~650 CommonMark tests and more than 1k extra tests confirmed with CM reference parsers"; "2300+ tests, 100% coverage, fuzz testing" (cargo-fuzz + honggfuzz); "100% safe Rust, also 100% safe HTML by default"; `#![no_std]` + alloc; extensions: GFM (autolink literal, footnote, strikethrough, table, tagfilter, task list), MDX (ESM, expressions, JSX), frontmatter, math; mdast AST via `to_mdast` — [GitHub](https://github.com/wooorm/markdown-rs).
- Security caveat in its own README: "It is also possible to crash `markdown-rs` with smaller payloads"; "It is wise to cap the accepted size of input" — [GitHub](https://github.com/wooorm/markdown-rs). No release number shown on the page; 38 open issues / 69 open PRs at fetch time, so maintenance cadence unclear — [GitHub](https://github.com/wooorm/markdown-rs).

**cmark / cmark-gfm (C reference)**
- cmark "passes all CommonMark conformance tests"; "extensively fuzz-tested using" AFL (`make fuzztest`, `make afl`, `make libFuzzer`); test suite "includes pathological cases that bring many other Markdown parsers to a crawl" (deeply nested brackets/blockquotes); scrubs raw HTML and `javascript:`, `vbscript:`, `data:`, `file:` links by default (`CMARK_OPT_UNSAFE`/`--unsafe` to allow); War and Peace in "127 milliseconds on a ten year old laptop"; "10,000 times faster than the original Markdown.pl" — [GitHub cmark](https://github.com/commonmark/cmark).
- cmark 0.31.2 is packaged in distros in 2026 (Arch build 2026-02-14) — [Arch](https://www.archlinux.org/packages/extra/x86_64/cmark).

**JavaScript engines**
- micromark: "tested with the ~650 CommonMark tests and more than 1000 extra tests"; markets itself as smallest CommonMark-compliant parser — [GitHub micromark](https://WWW.GITHUB.COM/micromark/micromark).
- markdown-it: "essentially CommonMark compliant", optional GFM features, `commonmark` preset — [GitHub markdown-it](https://github.com/markdown-it/markdown-it); ~9.3M weekly downloads vs ~2.2M for remark (snapshot) — [npm-compare](https://npm-compare.com/markdown-it,marked,micromark,remark,showdown).
- marked: described as not CommonMark/GFM-matching and unsafe by default; smallest bundle (≈12.5 KB vs micromark 15.4 KB, markdown-it 52.7 KB) — [TanStack comparison](https://tanstack.com/markdown/latest/docs/comparison); [deno-gfm issue](https://github.com/denoland/deno-gfm/issues/54). TanStack recommends commonmark.js, micromark or unified "when exact specification behavior is a requirement" — [TanStack](https://tanstack.com/markdown/latest/docs/comparison).
- Old JS speed data (dated): markdown-it ~986 ops/s vs marked ~729 on one input; author warned results are input-dependent — [talk.commonmark.org (old)](https://talk.commonmark.org/t/performance-of-commonmark-reference-implementations/16).

**pandoc**
- 3.8 released 6 Sep 2025 (new `xml` AST format); 3.9 (4 Feb 2026) adds alerts support to pandoc Markdown (not default) — [pandoc 3.8 changelog](https://hackage.haskell.org/package/pandoc-3.8/src/changelog.md); [pandoc releases](https://github.com/jgm/pandoc/releases). `--mathml` is a usable LaTeX→MathML path — [leancrew 2025](https://leancrew.com/all-this/2025/05/mathml-with-pandoc/).

**Typst HTML export**
- Typst 0.14.0 (24 Oct 2025): HTML export "remains experimental" but "now reflects the complete semantic meaning of your document without requiring any custom show rules"; requires `--features html` / `TYPST_FEATURES=html`; tracked in typst/typst#5512 — [Typst 0.14 changelog](https://typst.app/docs/changelog/0.14.0/); [Typst 0.14 blog](https://typst.app/blog/2025/typst-0.14); [forum](https://forum.typst.app/t/typst-0-14-now-available/6586); [HTML reference](https://typst.app/docs/reference/html/). typst-html crate at 0.14.2 — [docs.rs](https://docs.rs/crate/typst-html/latest/source/src/encode.rs).

**Djot / jotdown**
- Seven implementations: djot.lua (original reference, may lag), djot.js (focus of development), jotdown (Rust), godjot, djot-php, djota (Prolog), djoths — [jgm/djot](https://github.com/jgm/djot).

### Inferences
- "World-class" for an engine in 2026 = 100% of the ~652 CommonMark 0.31.2 examples (plus ~670 GFM examples if claiming GFM), fuzzing, no super-linear blowups on cmark's pathological suite, HTML/link scrubbing by default, and an opt-in extension set roughly equal to comrak's or pulldown-cmark's flag list.
- pulldown-cmark is the most natural "reference to port from" for a Rust std-only project: pure Rust, no unsafe, minimal deps, pull-event architecture close to a hand-written two-pass parser. comrak (cmark-gfm port with an arena AST) is the best oracle for GFM behaviour.
- Typst/Quarto/MyST are document systems, not Markdown engines in md2html's class; they matter as feature benchmarks (cross-refs, admonitions, math), not as parsers to copy.

### Gaps
- No current, independent head-to-head benchmark across pulldown-cmark/comrak/markdown-rs/cmark/md4c/markdown-it/marked was found; only competitor-authored or dated numbers.
- Current exact npm versions of markdown-it, marked, micromark/remark/rehype were not retrieved (background, unverified: markdown-it 14.x, marked 15–16.x, micromark 4.x, remark 15.x).
- MyST (mystmd) and Quarto (1.7/1.8) current versions and feature details were not fetched in this pass. Background (unverified): MyST uses `:::{note}` / ```` ```{directive} ```` colon/backtick fences and roles `{role}`content``; Quarto uses pandoc with `::: {.callout-note}` fenced divs.
- jotdown's own version/conformance status not fetched.
- pulldown-cmark's fuzzing setup was not stated in the README fetch (background, unverified: it has a fuzz directory and has fixed quadratic-time bugs found by fuzzing).

## 2. Which syntax extensions are now "expected" (and how md2html's choices compare)

### Takeaway
The de facto baseline is CommonMark + the GFM set (tables, task lists, strikethrough, autolinks, footnotes) + GitHub alerts (`> [!NOTE]` with five fixed types). A second tier — math, heading IDs/attributes, front matter, definition lists, super/subscript, wikilinks, emoji shortcodes — is supported as opt-in flags by both leading Rust engines, so it is "expected to be available" rather than "expected on by default". md2html's `> Note:` callout syntax is non-standard; adopting `> [!NOTE]` (and keeping `> Note:` as a legacy alias) aligns with GitHub, pulldown-cmark, comrak and pandoc 3.9.

### Cited Findings
- GitHub alerts syntax: blockquote whose first line is `[!TYPE]`; exactly five types: NOTE, TIP, IMPORTANT, WARNING, CAUTION. GitHub guidance: one or two per article, avoid consecutive alerts, "Alerts cannot be nested within other elements." — [GitHub Docs](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).
- Alerts are now implemented in pulldown-cmark (ENABLE_GFM) — [docs.rs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html); comrak ("Alerts") — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0); pandoc Markdown 3.9 (opt-in) — [pandoc releases](https://github.com/jgm/pandoc/releases).
- GitHub footnotes: `[^1]` reference, `[^1]: text` definition anywhere in the file; rendered at the end regardless of position — [GitHub Docs](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).
- GitHub task lists: `- [ ]` / `- [x]` — same source.
- GitHub emoji shortcodes `:+1:`; heading anchors auto-generated; raw HTML allowed for `<sub>`, `<sup>`, `<ins>`, `<kbd>`, `<picture>`, `<br/>`; HTML comments hidden — same source.
- Front matter: pulldown-cmark supports both YAML `---` and TOML-style `+++` metadata blocks (as raw text events — parsing YAML is left to the caller) — [docs.rs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html); comrak "Front matter" — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0); markdown-rs "frontmatter" — [GitHub](https://github.com/wooorm/markdown-rs).
- Heading attributes `{#id .class key=val}`: pulldown-cmark ENABLE_HEADING_ATTRIBUTES; comrak "Header IDs" (auto IDs) — sources above.
- Math `$…$`/`$$…$$`: pulldown-cmark ENABLE_MATH, comrak Math, markdown-rs math — all emit raw TeX for a downstream renderer — sources above.
- Definition lists: pulldown-cmark (commonmark-hs-extensions syntax: term line then `: definition`), comrak "Description lists" — sources above.
- Super/subscript: pulldown-cmark ENABLE_SUPERSCRIPT/SUBSCRIPT, comrak Superscript — sources above.
- Wikilinks `[[Page]]`: pulldown-cmark (Obsidian-style), comrak — sources above.
- Smart punctuation: pulldown-cmark ENABLE_SMART_PUNCTUATION converts dashes, ellipses, straight quotes — [docs.rs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html).
- Raw HTML policy: cmark, cmark-gfm, comrak and markdown-rs all scrub raw HTML and dangerous URL schemes by default and require an explicit unsafe flag — [cmark](https://github.com/commonmark/cmark); [comrak](https://docs.rs/crate/comrak/0.59.0); [markdown-rs](https://github.com/wooorm/markdown-rs). comrak removed GFM tagfilter in 0.56.0 — [comrak](https://docs.rs/crate/comrak/0.59.0).
- GitHub Docs basic-syntax page does not document Mermaid or math — [GitHub Docs](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).

### Inferences
- Priority list for md2html, ordered by user expectation vs std-only cost:
  1. Nested lists (CommonMark core; the single biggest gap — not an extension at all).
  2. Inline images, `***bold-italic***`, correct bracket handling (core CommonMark; the "bare [brackets] misparsed as links" bug is exactly the reference-link locality problem CommonMark solves by only treating `[x]` as a link when a matching definition exists).
  3. Task lists, strikethrough, autolinks, footnotes (GFM; each ~50–200 lines std-only).
  4. `> [!NOTE]` alerts with the five GitHub types (trivial if `> Note:` already exists; keep the old form as an alias). The five types map onto md2html's Note/Tip/Warning plus Important/Caution.
  5. Heading `{#id}` attributes, YAML/TOML front matter (title/description only — a full YAML parser is not needed for flat `key: value`), smart punctuation (small, std-only).
  6. Math — conflicts most with zero-dependency philosophy (see section 4).
  7. Definition lists, super/subscript, wikilinks, emoji shortcodes: low priority; emoji shortcodes need a ~1,800-entry table (size cost, not code cost).
- Raw HTML: since md2html produces a local, self-authored page, passing through a small allowlist (`<details>/<summary>`, `<kbd>`, `<sub>`, `<sup>`, `<br>`, `<mark>`, comments hidden) mirrors GitHub's visible behaviour without needing a sanitizer; escaping everything else matches the current "shown verbatim" philosophy and the cmark safe-by-default posture.
- Mermaid ` ```mermaid ` blocks: rendering requires the mermaid JS library (~MBs), which conflicts with the offline single-file goal unless inlined; leaving them as code blocks with a `language-mermaid` class is the honest std-only behaviour.

### Gaps
- Exact syntax/semantics of GitHub's `<details>` handling and Mermaid rendering on GitHub were not fetched (GitHub docs for diagrams live on a separate page).
- Abbreviations (`*[HTML]: ...`, PHP Markdown Extra / pandoc) support in the Rust engines not confirmed; neither pulldown-cmark nor comrak lists it.

## 3. Syntax highlighting: with and without dependencies

### Takeaway
All serious options (syntect, tree-sitter via inkjet/arborium, shiki) are heavy dependencies with large grammar data; there is no std-only highlighter of note. A tiny built-in, regex-free keyword/string/comment/number tokenizer for a handful of languages is feasible but is a separate mini-project; the zero-JS, CSS-class output pattern (arborium/comrak style) is the one to copy.

### Cited Findings
- comrak delegates highlighting via a `SyntaxHighlighterAdapter` trait with a default syntect plugin; CLI `--syntax-highlighting` defaults to `css` (class-based) — [docs.rs comrak](https://docs.rs/crate/comrak/0.59.0).
- arborium (2.x, latest seen 2.18.1): tree-sitter grammar collection with HTML rendering and WASM; per-language feature flags (e.g. `lang-rust`); HTML output wraps tokens in custom elements colored by a theme stylesheet; 32 built-in themes; also a browser script/ESM/npm package — [docs.rs arborium 2.18.1](https://docs.rs/crate/arborium/2.18.1); [README 2.12.1](https://docs.rs/crate/arborium/2.12.1/source/README.md).
- inkjet (0.10.x): "batteries-included" tree-sitter highlighting; grammars must check in generated parsers and need a base `highlights.scm`; queries largely from Helix — [inkjet README](https://docs.rs/crate/inkjet/0.10.4/source/README.md).

### Inferences
- Any tree-sitter route pulls in C grammars compiled via `cc` — incompatible with "std only, no crates". syntect bundles Sublime grammars and onig/fancy-regex — also incompatible.
- Std-only cost estimate (inference, not measured): a generic lexer recognizing line/block comments, strings with escapes, numbers and a per-language keyword list, emitting `<span class="k|s|c|n">`, is roughly 200–400 lines plus ~10–30 lines of keyword data per language (Rust, Python, JS/TS, shell, JSON, TOML/YAML, HTML/CSS, C). It will be wrong on edge cases (raw strings, heredocs, template literals, nested comments) and must degrade to plain text, never mis-render.
- A build-time option (shiki/arborium run once, inline output) contradicts "one binary turns one file into one page" unless offered as an external optional preprocessing step.

### Gaps
- autumnus (Rust tree-sitter highlighter, used by MDEx/Elixir per background knowledge) did not appear in search results; unverified.
- shiki's current version and binary size not fetched; tree-sitter-highlight crate status not fetched.
- No measured size/line counts for minimal highlighters were found.

## 4. Math rendering for an offline single-file page

### Takeaway
MathML Core has shipped in all three engines (Chromium since 109, Jan 2023; Safari and Firefox long before), so emitting `<math>` directly is the zero-JS, offline-friendly path. The Rust converter of choice is math-core (0.9.0, Oct 2026; MathML Core target, fork of latex2mathml), but it has 12 dependencies, so std-only would mean writing a mini LaTeX→MathML translator. Visual quality still depends on a math font and a small CSS fixes sheet.

### Cited Findings
- caniuse lists MathML supported in Chrome 109–142+, Safari 10–26.2 and TP, Firefox 4–145, Firefox Android from 144 — [caniuse MathML](https://caniuse.com/mathml). Chrome 109 release date 10 Jan 2023 — [web-features explorer](https://web-platform-dx.github.io/web-features-explorer/features/mathml). Chromium 109+ specifically implements MathML Core — [w3cub caniuse mirror](https://docs.w3cub.com/browser_support_tables/mathml). A "MathML Full" web-feature entry exists separately — [web-features: mathml-full](https://web-platform-dx.github.io/web-features-explorer/features/mathml-full).
- One personal page still claims Chromium lacks MathML; it contradicts caniuse/web-features and appears outdated — [proyecto-eden](https://proyecto-eden-3-esferas.github.io/mathml.browser-support.html).
- math-core: converts LaTeX to MathML Core, "the MathML specification that is being implemented by web browsers"; aims at "at least those [commands] that KaTeX supports" but not all yet; 0.8.1 (2026-08-30), 0.9.0 (2026-10-12); 12 normal deps (hashbrown, memchr, phf, rustc-hash, serde, strum, …); originally forked from latex2mathml — [docs.rs math-core 0.8.1](https://docs.rs/crate/math-core/0.8.1). MSRV 1.91 at 0.5.1 — [math-core 0.5.1](https://docs.rs/crate/math-core/0.5.1).
- Browser caveats per math-core: Chromium ignores `ssty` glyph variants (primes in scripts), Safari mis-spaces accents, Chromium/Safari don't center certain accents, Chromium lacks `<menclose>` (`\cancel`, `\sout` need polyfill); include its `mathmlfixes.css` — [docs.rs math-core 0.8.1](https://docs.rs/crate/math-core/0.8.1).
- Fonts: recommends New Computer Modern Math Book, Libertinus Math, Noto Sans Math; a subset NCM Math is ~450 kB WOFF2 (~200 kB without `ssty`, hurting Firefox) — [docs.rs math-core 0.8.1](https://docs.rs/crate/math-core/0.8.1).
- Alternatives: pulldown-latex (Rust lib only; can't strip redundant grouping; as of June 2025 no `\mathcal`/`\mathscr` distinction); Temml (JS KaTeX fork, MathML-only, "much more willing to work around browser bugs"); LaTeXML (Perl, used by arXiv); latex2mathml (pure Rust) — [docs.rs math-core 0.8.1](https://docs.rs/crate/math-core/0.8.1); [Temml on ecosyste.ms](https://awesome.ecosyste.ms/projects/github.com%2Fronkok%2Ftemml).
- pulldown-cmark/comrak/markdown-rs only *recognize* `$…$`/`$$…$$` and hand TeX to the renderer — [pulldown-cmark Options](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html).

### Inferences
- KaTeX/MathJax inlined into a single page means hundreds of KB to MBs of JS+fonts and a runtime script — the opposite of md2html's philosophy. MathML Core output needs no script.
- Std-only tiers: (a) recognize `$…$`/`$$…$$`, escape, and wrap in `<code class="math">` — zero risk, keeps source readable; (b) a hand-written subset translator (identifiers → `<mi>`, numbers → `<mn>`, operators → `<mo>`, `^`/`_` → `<msup>/<msub>/<msubsup>`, `\frac` → `<mfrac>`, `\sqrt` → `<msqrt>/<mroot>`, Greek letters and ~150 common symbol commands, `\left…\right`, simple `matrix`/`cases`) — plausibly 600–1,200 lines (inference, not measured); (c) port of latex2mathml-style logic. Tier (b) covers most notes/README math; fall back to (a) on any unknown command rather than guess.
- Embedding a math font (~200–450 kB) is optional; system math fonts (Cambria Math, STIX Two, Latin Modern) usually exist — so default to no embedded font.

### Gaps
- latex2mathml (Rust) current version and dependency count not fetched (the CERN mirror hit is the Python package of the same name, 3.81.0 — not the Rust crate).
- No 2026-specific browser MathML release notes found; caniuse snapshot taken at face value.
- KaTeX/MathJax current versions and bundle sizes not fetched.

## 5. The CommonMark spec test suite as a conformance harness

### Takeaway
The spec ships as `spec.txt` with ~650 embedded examples (652 in 0.31.2 per comrak's badge); `test/spec_tests.py --dump-tests` exports them as JSON (`markdown`, `html`, `section`, `number`), which a std-only Rust project can vendor as a static file and loop over in `cargo test`, reporting pass counts per section and gating on a ratchet.

### Cited Findings
- Spec examples are fenced blocks tagged `example` with Markdown input, a separator line, and expected HTML; "over 500 embedded examples which serve as conformance tests" — [commonmark-spec](https://github.com/commonmark/commonmark-spec).
- Run against any executable: `python3 test/spec_tests.py --program $PROG`; dump: `python3 test/spec_tests.py --dump-tests` → JSON objects with `markdown`, `html`, `section`, `number`; the `commonmark-spec` npm package exports the same `tests` array — [commonmark-spec](https://github.com/commonmark/commonmark-spec).
- 0.31.2 counts: 652 CommonMark examples; GFM spec 670 — [comrak badges](https://docs.rs/crate/comrak/0.59.0). markdown-rs and micromark run "~650 CommonMark tests" plus 1k+ extra cases "confirmed with CM reference parsers" — [markdown-rs](https://github.com/wooorm/markdown-rs); [micromark](https://WWW.GITHUB.COM/micromark/micromark).
- CommonMark 0.31.2 announcement: "most of the changes to the spec are very minor" — [talk.commonmark.org](https://talk.commonmark.org/t/ann-commonmark-0-31-2/4591). (Release date Jan 2024: background, unverified.)
- cmark also ships pathological tests (deep nesting of brackets/blockquotes) — [cmark](https://github.com/commonmark/cmark).

### Inferences
- Harness recipe for md2html (std-only): vendor `spec.json` (≈200 KB) under `tests/`; a tiny hand-written JSON reader (or pre-convert to a simple delimited text format at vendor time) feeds a test that renders each example's `markdown` with a "fragment mode" (no page chrome) and compares to `html` after normalizing whitespace between tags (spec_tests.py normalizes HTML before comparing — background). Print "N/652 pass, by section" and fail the test only if the count drops below a checked-in baseline (ratchet). This lets a deliberate subset coexist with honest measurement.
- Add a pathological-input test (e.g. 50,000 nested `[` or `>` or `*`) asserting runtime < 1 s to catch quadratic behaviour.
- A subset parser can still claim "N/652" truthfully; md2html's "show unknown constructs verbatim" policy will intentionally fail sections like setext headings, indented code, HTML blocks, link reference definitions unless implemented.

### Gaps
- spec_tests.py's exact normalization rules and `--pattern` flag not confirmed in the fetched README.
- cmark's `pathological_tests.py` contents not fetched.

## 6. Djot and "fixed Markdown": lessons on ambiguity

### Takeaway
Djot (by CommonMark's author) identifies precisely the areas where md2html has bugs — brackets/reference links, emphasis, and nesting — and fixes them by making syntax local, linear-time and uniform. For a hand parser, Djot's rules are a good model for *where to be strict* even while staying Markdown-compatible.

### Cited Findings
- CommonMark emphasis "resolved by a daunting list of 17 rules"; Djot uses `_` emphasis and `*` strong, no doubled characters — [jgm/djot](https://github.com/jgm/djot).
- Reference links made "local"; shortcut links `[like this]` removed; reference labels no longer case-insensitive; non-locality "makes accurate syntax highlighting nearly impossible" — [jgm/djot](https://github.com/jgm/djot).
- "It should be possible to parse djot markup in linear time, with no backtracking"; unclosed code spans run to end of paragraph; parsers should not need Unicode character classes, HTML tag recognition, or entities — [jgm/djot](https://github.com/jgm/djot).
- No indented code blocks; no setext headings; block elements can't interrupt paragraphs; sublists must be preceded by a blank line; "anything that is indented beyond the start of the list marker belongs in the list item"; `>` must be followed by space/newline; raw HTML only in explicitly marked contexts; generic attribute syntax for any element — [jgm/djot](https://github.com/jgm/djot).

### Inferences
- md2html's "bare [brackets] misparsed as links" bug is the shortcut-reference-link ambiguity. Fix in CommonMark style: `[text]` becomes a link only if followed by `(…)` or matching a defined reference label; otherwise literal. Djot's view supports not implementing shortcut links at all.
- Nested lists: Djot's rule ("indented beyond the start of the list marker belongs in the item", blank line before sublists) is far simpler to hand-implement than CommonMark's list-item width rules, but would fail some CommonMark examples; CommonMark's rule (content column = marker width + following spaces, 1–4) is the compatible choice and well specified.
- Emphasis: implementing CommonMark's delimiter-run algorithm (left/right-flanking + the "process emphasis" stack with the multiple-of-3 rule) is the only way to get `***bold-italic***` and intraword cases right; it is ~200–300 lines and linear with the spec's openers-bottom optimisation (background). A simpler ad-hoc scanner is what produces the current gaps.
- Adopt Djot-like attributes `{#id .class}` only on headings (as pulldown-cmark does) to avoid broader ambiguity.

### Gaps
- jotdown's conformance and performance numbers not fetched.
- No data on Djot adoption levels in 2026.

## 7. Recommendation summary: can a std-only hand parser stay competitive?

### Takeaway
Yes on output quality and philosophy, provided md2html (a) re-architects its parser around the CommonMark two-phase model (block structure with a container stack, then inline parsing with a delimiter stack), (b) measures itself against the 652 spec examples with a ratchet, and (c) adds the GFM set plus alerts. Porting is better framed as "reimplement the CommonMark algorithms, using pulldown-cmark/comrak as reference oracles" than as copying a crate. Matching comrak's full extension breadth is neither needed nor wise.

### Cited Findings
- The leading engines' extension sets are opt-in and individually enabled — [comrak](https://docs.rs/crate/comrak/0.59.0); [pulldown-cmark](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html).
- Even mature engines warn about DoS on crafted input (markdown-rs: cap input size) — [markdown-rs](https://github.com/wooorm/markdown-rs); cmark keeps a pathological test suite — [cmark](https://github.com/commonmark/cmark).
- pulldown-cmark proves a pure-Rust, no-unsafe, allocation-light parser can lead speed charts (~275 MiB/s in a third-party bench) — [ferromark README](https://docs.rs/crate/ferromark/0.1.4/source/README.md); [pulldown-cmark README](https://cdn.jsdelivr.net/gh/pulldown-cmark/pulldown-cmark@main/README.md).

### Inferences
- Speed is irrelevant for one-file CLI use; correctness, predictability and no pathological blowups matter. A std-only parser can match these.
- Estimated size (inference): a CommonMark-faithful block+inline parser is ~3,000–5,000 lines in Rust (pulldown-cmark's core parser is of this order — background, unverified); md2html at ~2,200 lines total would roughly double. GFM tables/tasks/strike/autolinks/footnotes add ~600–1,000 lines.
- Pragmatic path: keep "deliberate subset" as a documented profile, but switch the two algorithms that cause the known bugs (list container stack; delimiter-stack emphasis + bracket resolution) to CommonMark's, then let the spec ratchet decide which further sections to admit. Use differential testing against pulldown-cmark or comrak outside the build (e.g. a script run by maintainers) to avoid adding a dependency.
- Highest conflict with zero-deps: syntax highlighting (tree-sitter/syntect) and math rendering beyond a subset. Both have honest std-only fallbacks (class-tagged code; MathML subset or literal TeX).

### Gaps
- No published line counts for pulldown-cmark/comrak parser cores were retrieved to firm up the size estimate.
- No source found on any std-only Rust Markdown parser achieving full CommonMark compliance (ferromark's dependency posture not checked).
