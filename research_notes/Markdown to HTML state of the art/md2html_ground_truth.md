# md2html ground truth (verified against the code, v0.3.3, 2026-11-21)

This describes md2html 0.3.3, before 0.4.0 addressed most of it; see CHANGELOG.md for what changed.

Verified by reading src/ and rendering probe files with target/release/md2html. Treat this as authoritative over any researcher's assumption about what md2html lacks.

## Already present (do NOT recommend as new)
- Scrollspy: the outline highlights the current chapter/section (`mark()`/`spy()` in src/page.rs, scroll listener using offsetTop), opens the current chapter's sub-list, scrolls the outline to keep it in view, and shows a breadcrumb ("Chapter › Section") in the top bar (`#crumb`).
- Reading progress bar (`#bar`, width from scroll fraction).
- Search: `<dialog>` modal, Cmd/Ctrl-K and `/` open it, arrow keys + Enter, ranked by title/body hits and phrase bonus, snippets with `<mark>` highlighting in results, top 14 hits. On jump, the target section gets a "landing glow". It does NOT highlight matches inside the page body after the jump.
- Heading `#` anchors with aria-label; skip link (`.skip`); `<nav aria-label="Contents">`; `lang="en"` hardcoded.
- Print stylesheet exists but is minimal: `@media print{.bar,.copy,.skip{display:none}; white background; black text/links}`. No link-URL printing, no break-inside rules, no expansion of glossary, outline nav not explicitly hidden.
- Copy button on code blocks (navigator.clipboard).
- Theme light/dark/system, text size 85–200%, outline hide/drawer; prefs in localStorage with try/catch. Outline key is `iv-toc` (legacy name from the iv handbook) and is shared by every md2html page on the same origin.
- Figure zoom overlay (custom JS, figure.js), only shipped when the page has a figure.
- Glossary hover links, foot-of-page entries with "back" links, shared glossary across pages.
- `<meta name="robots" content="noindex,nofollow">` is always emitted.
- Output size: README.md renders to ~68 KB; a near-empty page is ~36 KB (CSS+JS baseline). Binary ~460 KB (release, LTO, stripped).

## Verified gaps and bugs (from probe rendering)
- SECURITY: link URLs are not scheme-filtered. `[x](javascript:alert(1))` -> `href="javascript:alert(1"` and `[b](data:text/html,hi)` -> `href="data:text/html,hi"`. Quotes ARE escaped (`&quot;`), so no attribute injection. Raw HTML is escaped (`<script>` -> `&lt;script&gt;`), so raw HTML is never passed through.
- Parentheses inside a URL end the URL early (`alert(1)` -> `alert(1`, the `)` leaks into text).
- Nested lists flatten (`  - nested` becomes a sibling item).
- Inline image `![c](...)` inside a paragraph renders as a literal `!` plus a link.
- Not supported, shown verbatim: footnotes `[^1]`, task lists `- [ ]`, strikethrough `~~x~~`, math `$x^2$`, GitHub alerts `> [!NOTE]` (it becomes a generic Note callout with the literal text "[!NOTE]"), front matter, definition lists, heading IDs `{#id}`.
- `***bold italic***` mis-nests: `<strong>*bi</strong>*`.
- No syntax highlighting (the code fence language is only named).
- No Content-Security-Policy meta, no description/Open Graph/theme-color/color-scheme meta, no favicon.
- Leftover handbook wording on generic pages: the glossary chapter is titled "The words in this book" in the outline, and search says "Nothing in the book matches that."
- Rendering date is stamped in the page header (the "eyebrow"), so output is not byte-reproducible across days.
- No stdin/stdout, no batch/directory mode, no watch, no config, no library API, no lint/check mode; one file per run.
- About 10 unit tests (src/main.rs `mod tests`); no CommonMark conformance harness, no golden files, no fuzzing, no CI config in the repo.
- Images are linked, not embedded (no --embed option).

## Philosophy constraints (owner's stated design)
- Standard library only, no crates; one offline self-contained HTML file; opinionated "handbook" renderer, not a general Markdown converter; unknown syntax shown verbatim rather than guessed.
- Primary producers of the Markdown are often AI coding agents writing handbooks/reports on the owner's machine.
