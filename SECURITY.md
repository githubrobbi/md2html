# Security

md2html turns Markdown, possibly written by someone else, into a page that other people open. It is built so that it cannot be used to make a page that does harm.

## What md2html guarantees

1. **A page runs only md2html's own code.** No script, event handler, `javascript:` or `vbscript:` URL, frame, plug-in, form, meta refresh or `<base>` from the Markdown reaches the page, in any mode. Raw HTML is shown as text except a short list of harmless tags; with `--html`, rich HTML (images, links, video, SVG, tables, layout) passes through an allowlist that removes everything executable. The author's `head` file keeps only `meta` (without `http-equiv`), `link` (stylesheets, icons and the like) and `style`.
2. **A document reads nothing it should not.** Files named in the front matter must lie inside the document's folder and be of their kind (`.css`, `.html`, `.js`, `.md`). An image is embedded only from the document's folder or the folder md2html runs in, and only when its bytes are an image. Only the command line, which the person running md2html controls, may name files elsewhere.
3. **Hostile input cannot stall or crash it.** Parsing is linear in the input; nesting of emphasis, links and images is capped at 64 levels; inputs over 64 MB and images over 32 MB are refused (`--max-input`, `--max-embed` raise these on the command line only). The test suite renders pathological inputs (100,000 asterisks, brackets, quote marks; tens of thousands of footnotes and link definitions) under a time limit on every change.
4. **It writes only where it is told.** It never overwrites its input or writes a Markdown file, and writes each output whole or not at all.
5. **The local server stays local.** `--serve` listens on 127.0.0.1 only, answers only requests addressed to this machine's own names (so a web page elsewhere cannot reach it by DNS rebinding), serves only files inside the document's folder (no hidden files, no way out by `..` or its encodings), and only to GET and HEAD.
6. **A live page talks only to where it came from.** `--live` lets a page fetch the first kilobyte of its own URL to see whether it changed; with `--csp` the policy allows exactly that origin and nothing else.
7. **PDF printing is offline.** `--pdf` runs Chrome headless with a fresh, throwaway profile, a time limit, and every network connection refused.
8. **No third-party code.** The binary is the Rust standard library and md2html; the toolchain is pinned, and `cargo deny` and `cargo vet` gate any dependency (see [docs/SUPPLY-CHAIN.md](docs/SUPPLY-CHAIN.md)).

What md2html does not promise: it does not judge what a page *says*. A page can still carry a misleading link to an ordinary web address, as any document can; `--csp` additionally forbids the page from connecting anywhere.

## Reporting a problem

Please report a way around any of the guarantees above privately, through GitHub's private vulnerability reporting on this repository (Security, then "Report a vulnerability"), with a Markdown file that shows it. Fixes are released as a new version with a CHANGELOG entry.
