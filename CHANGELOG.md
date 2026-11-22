# Changelog

What changed in each version, newest first.

## 0.5.0 — 2026-11-22

Diagrams, and the supply chain vetted.

- Mermaid flowcharts (`flowchart` and `graph`, every direction, subgraphs, all node shapes, solid, dotted, thick and invisible edges, arrow, circle and cross ends, labels in every form) and sequence diagrams (participants and actors, every arrow, activations, notes, `loop`, `alt`, `opt`, `par`, `critical`, `break`, `autonumber`) are drawn by md2html itself as SVG inside the page. No script, nothing downloaded, the page's colours in both themes and in high contrast, words that search finds, and a title and description for screen readers.
- The layout is the layered one: cycles broken, nodes ranked, long edges routed through waypoints, ranks ordered to cut crossings with subgraphs kept together, nodes placed toward their neighbours; edges meet boxes on the face toward them; long labels wrap.
- Every Mermaid diagram found in Markdown on this machine draws, the medical report's decision chart and the interview handbook's sequence diagram among them.
- `--mermaid-js FILE` (or `mermaid_js:`) puts a local mermaid.js into the pages that have a diagram md2html does not draw (pie, Gantt, class...), and `mermaid: js` sends every diagram to it; with `--csp`, such a page allows inline styles, which mermaid.js writes. `mermaid: code` shows every diagram as code. Without mermaid.js, another kind stays code, and `--check` says so.

### Supply chain

- The toolchain is pinned to the latest stable, Rust 1.100.0 (`rust-toolchain.toml`); CI installs from it.
- `cargo deny` and `cargo vet` run in CI on the (empty) crate graph; `deny.toml` and `supply-chain/` are the gate a first crate would meet.
- The browser test tools moved to the latest releases, playwright-core 1.65.1 and axe-core 4.14.1, after vetting: publisher, signatures, SLSA provenance, no dependencies, no install scripts, no advisories. CI verifies the signatures on every run.
- The CI actions moved to checkout v7.0.1 and setup-node v7.1.0, pinned to their GitHub-verified commits; Node 26 LTS; checkout keeps no credentials.
- Dependabot proposes updates weekly after a seven-day cooldown. `docs/SUPPLY-CHAIN.md` records every dependency and its evidence.
- The newer axe-core found that the search button's name left out its visible text (WCAG 2.5.3); fixed.

## 0.4.1 — 2026-11-22

- A figure clicked before its lazy image had loaded opened as a thin strip: the enlargement now takes the image's shape once it loads. The click-through checks the shape.
- A displayed formula wider than a phone takes the keyboard's focus, like a wide table or code block.
- `docs/report.md`: everything md2html does, applied on one page and rendered by md2html (`tests/browser/shots.js` takes its screenshots). The accessibility audit also reads pages that carry a Content-Security-Policy.

## 0.4.0 — 2026-11-21

A full Markdown reader under the same page.

### Reads all of Markdown

- A new parser, standard library only, built the way CommonMark specifies: blocks as nested containers, inline emphasis and links from a delimiter stack. It passes 640 of the 652 examples of CommonMark 0.31.2, and the tests keep that count from falling.
- Nested lists, quotations (and quotes inside quotes), indented code, `~~~` fences, setext headings, reference links, autolinks, hard line breaks, backslash escapes and entities.
- GitHub's additions: task lists, strikethrough, bare URLs, footnotes, table column alignment, and the five alerts `> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`.
- `***bold italic***`, images inside a sentence, parentheses inside a URL, and `[V]`-style brackets as plain text: the gaps the README listed are closed.
- Maths: `$...$`, `$$...$$` and ` ```math ` typeset as MathML, which every current browser draws without a script. Dollar amounts stay dollar amounts.
- Code coloured for some twenty languages and formats, with no grammar engine.
- Harmless HTML passes (`<kbd>`, `<sub>`, `<details>`, `<br>` ...); anything else is shown as text unless `--html` trusts it.
- `{width=50%}` after an image sizes it; `{#id}` after a heading names its anchor.

### Settings

- Front matter at the top of the Markdown, and the same keys on the command line (`--set key=value`, `--title`, `--lang`, `--brand`, `--footer`, `--css`, `--head`): title, language, brand and its link, kicker, eyebrow, date, version, description, author, footer, extra stylesheet and head.
- `lang: de` puts the page's own words in German: search, glossary, callouts, footnotes, figure, theme and text-size controls, and the date.
- Meta description (from the lead by default), Open Graph title and description, theme colours for both themes, an icon, a generator tag. `index: true` lifts the default `noindex`.

### The command line

- Several files or a whole folder per run, `-o` for a page or a folder, `-` for stdin and stdout.
- `--check` writes nothing and lists what to fix, with the line: anchors that are not on the page, missing files, footnotes without notes, raw HTML shown as text, unknown settings. It exits 1 when there is anything.
- `--watch` renders again on every save. `--nav` links the pages of one run with previous and next. `--fragment` writes only the HTML of the Markdown.
- `--embed` puts the images inside the page. `--csp` adds a Content-Security-Policy that lets only the page's own scripts and styles run.
- A link to a Markdown file beside the page goes to its HTML twin. Pages are written whole or not at all.

### The page

- Footnotes explain themselves on hover, like glossary words, and lead back to where you were.
- The words a search found are lit where it lands; the search is announced to screen readers and works as a combobox.
- A real print stylesheet: controls gone, link addresses printed, nothing cut across a page.
- Escape takes a word's explanation away; wide tables and code take the keyboard's focus; high-contrast mode is respected; the article is the page's main landmark.
- A page without chapters gets no empty outline; a document without a `# Title` gets one from its file name; several `#` headings become chapters.

### Accessible

- An audit with axe-core (WCAG 2.0 to 2.2, A and AA, and best practice) finds nothing on the README, the showcase, the docenta handbook, and the plan, research and family pages built on this machine, in both themes and at phone width.
- Every colour meets 4.5:1: code comments, numbers, strings and removed lines, and the tip's label, were darker than they looked.
- Links in running text are underlined, so colour is not the only sign of them.
- Callouts are notes inside the article, not landmarks of their own; the bar is the page's banner; the search button keeps its name on a phone; a task's box says whether it is done; each wide table's region has a name of its own; an empty header cell is a plain cell.
- Long words and bare URLs break instead of pushing the page sideways on a phone.
- Escape always closes the search, however it was opened.
- The outline's drawer slid for 1.4 s (it borrowed the landing glow's time); it takes 0.18 s, and print never catches it half-way.
- A click-through in Chrome (`tests/browser`) and the audit run in CI.

### Fixed

- A page with many bare brackets could make the renderer grow without bound (one run reached 38 GB). Rendering is now linear: the 261 KB docenta handbook takes 0.07 s, where it took over two minutes.
- `javascript:` and `data:` links were written into the page. They are now removed, however they are spelled.
- Code spans that held a `<` or crossed a line were split apart; bold runs could invert after a callout's label.
- The glossary's outline entry appeared on pages without a glossary; the search said "the book" on every page.

### Changed

- A quotation without a callout word is a quotation now, not a note. One that opens with a bold lead (`> **For Esther.** ...`) is still boxed as a note.
- The glossary is titled "Words used here", and the search says "Nothing on this page matches that."
- Anchors keep letters beyond ASCII (`über-die-lage`, not `ber-die-lage`).
- The docenta-specific glossary aliases are gone; a glossary's `Also` column names other spellings for any page.

## 0.3.3 and earlier

See the git history.
