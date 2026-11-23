<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# How md2html is built

md2html is one binary crate, `crates/md2html`, written against the standard library alone. Its structure is in its modules: twenty of them under `src/`, standing in layers. This page names the layers and the directions a module may use; a test holds the code to it.

## Layers

| Layer | Modules | What they hold |
| --- | --- | --- |
| 1. Foundation | `scan`, `text`, `model`, `input`, `watch` | Reading text without an index to get wrong; escaping and its reverse, URLs, encodings, the date; the document tree; reading files; noticing that they changed |
| 2. Raw HTML | `sanitize` | Where a tag in the text ends, the short list of tags a page keeps, and with `--html` the author's own HTML, rich but never executable |
| 3. Reading | `inline`, `block`, `math`, `highlight`, `mermaid` | Markdown read into the tree, the way `CommonMark` reads it; the notations inside it: maths, code, diagrams |
| 4. Settings | `config` | Front matter, the settings, the languages and the interface's words |
| 5. Rendering | `glossary`, `render` | The tree as the article: chapters, figures, notes, the outline, glossary links |
| 6. Page | `page` | The page around the article: its style, scripts, bar, search index and foot |
| 7. Export | `export` | The EPUB and the PDF |
| 8. Document | `document` | One Markdown text as a page: its settings, glossary, language, the files beside it; every layer below, composed |
| 9. Running | `report`, `serve`, `cli` | How a run talks to the person and how it ends; the live server; the command line |

## Directions

- A module uses modules of its own layer or a lower one, never a higher one.
- Inside a layer the uses go one way: `text` uses `scan`; `block` uses `inline` (a link reference definition is read with the inline link grammar); `render` uses `glossary`; `cli` uses `serve`, and both use `report`.
- No uses close a circle, inside a layer or across layers.
- A use names the module it reaches (`crate::text::esc`): the crate root declares the modules and `main`, and re-exports nothing.

Settings stand above the readers although no reader uses them: a reader takes what it needs as arguments (`inline::Env`), and nothing below rendering needs the interface's words.

A helper two layers share lives in the lower one, or in the foundation: decoding character references is `text`'s, not `inline`'s, because the sanitizer, the diagrams and the EPUB decode them too. What the command line and the server share is a module of its own (`report`), not one of them.

## What holds it

`crates/md2html/src/layer_tests.rs` reads the table above and every production source file of md2html (tests and `#[cfg(test)]` items left out). It fails when a module is missing from the table or the table names one that does not exist, when a use reaches a higher layer, when uses close a circle, or when a path names no module (a root alias). It runs with the other tests: `cargo test -p md2html`, `just go`, the pre-push hook and CI.

To add a module, put it in the row of the layer it belongs to. To add a use the test refuses, move the item to the layer both sides can reach rather than the module to the table's other end; if a layer is wrong, change this table in the same pull request and say why.

## The workspace's crates

| Crate | Job | Used by |
| --- | --- | --- |
| `md2html` | The product | Nothing; it uses no crate either |
| `md2html-version` | The tooling's version and build stamp | The four tooling binaries below |
| `md2html-ci-pipeline` | `just go` and `just ship` | Nothing |
| `md2html-gen-hooks`, `md2html-gen-workflow` | The git hooks and the CI workflow, from `scripts/ci/gates.toml` | Nothing |
| `md2html-manifest-audit` | The manifest gate | Nothing |
| `md2html-dev-tools` | `just use-local` and `just release-local` | Nothing |

The product and the tooling never meet: no tooling crate is a dependency of md2html. Every crate is internal (`publish = false`); md2html's contract is its command line and the pages it writes. No crate uses `async` or a foreign function, and `unsafe` code is forbidden in the root `Cargo.toml`; a crate that needed any of them would start with a policy in `docs/policies/`.

md2html stays one crate. Its layers share one binary and one release, no part has a second user or a dependency to keep away from the rest, and a split would add manifests and a public surface for nothing.
