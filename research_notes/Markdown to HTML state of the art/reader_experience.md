# Reader experience in single-file Markdown-to-HTML output (state of the art, late 2026)

Scope note: research ran on 2026-11-21 with about 16 tool calls. Items marked **[verified]** come from a page fetched or searched this session, with its link. Items marked **[unverified: prior knowledge]** come from the researcher's training knowledge (cutoff mid-2026) and were not re-checked against a live page this session. Re-check them before quoting them as fact. A docenta search of past sessions (scope `chats`) found no earlier competitive research on reader features for md2html. "sidenotes" and "scrollspy" were unseen terms in that corpus.

md2html baseline, as given in the assignment: left outline (hideable or hover drawer), Cmd/Ctrl-K search over an embedded index, light/dark/system theme, text size 85-200%, glossary with dotted hover links, foot-of-page entries and back-links, and a cross-page glossary, lead paragraph, "How to read this" cards, figures with captions and click-to-zoom that follows the pointer, callouts, copy buttons on code, # anchors on headings, and preferences kept in localStorage. Images are linked, not embedded.

## Survey: what reader-facing features do the leading tools offer?

### Takeaway
The richest single-document reading features come from Quarto (hover popups for footnotes, citations and cross-references, margin notes, code folding, tabsets, and true `embed-resources` single files) and from Gwern.net and Tufte CSS (sidenotes, recursive link popups, collapsible sections, reader mode). The docs-site generators (MkDocs Material, VitePress, Starlight, mdBook, Docusaurus) add navigation features md2html can copy within one page: scrollspy that follows the reader, the URL tracking the current section, back-to-top, keyboard shortcuts, a print page, and tuned search ranking. md2html already matches or beats most tools on glossary and tooltip handling.

### Cited Findings

**Pandoc (standalone / `--embed-resources`)**
- `--embed-resources` produces a standalone HTML file that should need no external files, using `data:` URIs for linked scripts, stylesheets, images and videos. Absolute URLs are downloaded. Elements with `data-external="1"` are left alone. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- Limits of `--embed-resources`: resources that JavaScript loads at run time cannot be embedded, and fonts may be missing with `--math-method=mathjax`. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- SVG images become `data:` URIs in `<img>`, or inline SVG when the image has the `inline-svg` class. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- `--self-contained` is a deprecated synonym for `--embed-resources --standalone`. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- Math: the default `--math-method` is now `mathml`, which is native in major browsers and needs no JavaScript or fonts. KaTeX and MathJax load from a CDN unless given a URL. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- `--reference-location` takes `block`, `section` or `document` (the default) and places footnotes in HTML. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- `--syntax-highlighting` takes `default`, `none`, `idiomatic` (a format-specific highlighter), a style name (pygments, kate, monochrome, breezeDark, espresso, zenburn, haddock, tango) or a `.theme` JSON file. `--highlight-style` and `--no-highlight` are deprecated. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- Other options: `--number-sections` (with `--number-offset`, which only affects HTML and Docx), `--section-divs` (wraps sections in `<section>`), `--email-obfuscation`. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)

**Quarto HTML**
- `embed-resources: true` writes one HTML file that inlines scripts, stylesheets, images and videos as `data:` URIs. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- TOC options: `toc`, `toc-depth` (default 3), `toc-expand` (default 1, auto-expands on scroll), `toc-title`, and `toc-location` (`left`, `right` (default), `body`, `left-body`, `right-body`). **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Hover popups: `footnotes-hover`, `citations-hover` and `crossrefs-hover` are all on by default. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- `anchor-sections: true` shows anchor links on hover over section titles. Figures and tables with cross-references also get anchor links. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Tabsets: `.panel-tabset`, where each top-level heading inside becomes a tab. `group="..."` syncs the chosen tab across tabsets, and `.active` sets the tab shown on load. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- External links: `link-external-icon`, `link-external-newwindow`, `link-external-filter`. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- `smooth-scroll` (off by default). Math methods: `mathjax` (default), `katex`, `webtex`, `gladtex`, `mathml`, `plain`. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- `minimal: true` turns off bootstrap theming, anchor sections, reference popups, tabsets, code copy and responsive figures, which shows what Quarto treats as its standard reader layer. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Margin content: `.column-margin` blocks, `reference-location: margin` (footnotes in the margin), `citation-location: margin`, `cap-location: margin` (figure and table captions in the margin), `[text]{.aside}` for un-numbered margin notes, and `.column-page`, `.column-screen` and `.column-body-outset` for wide content. **[verified]** — [Quarto article layout](https://quarto.org/docs/authoring/article-layout.html)
- Code: `code-fold` (`true` / `show`, using `<details>`), `code-summary`, `code-tools` (header menu with "Hide All Code", "Show All Code" and "View Source"), `code-overflow: wrap|scroll`, `code-line-numbers`, `code-annotations`, and adaptive light/dark `syntax-highlighting` (default `arrow`, with custom `light:`/`dark:` pairs). **[verified]** — [Quarto HTML code](https://quarto.org/docs/output-formats/html-code.html)
- Quarto also ships lightbox figures and comments (hypothesis, utterances, giscus). Comments need the network. **[verified, names only]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)

**MkDocs Material (now succeeded by Zensical)**
- The site shows a banner naming Zensical as the successor project. **[verified]** — [MkDocs Material tooltips](https://squidfunk.github.io/mkdocs-material/reference/tooltips/)
- Improved tooltips (`content.tooltips`, experimental, 9.5.0 or later) replace native `title` rendering for links, permalinks, the copy button and nav ellipses. Abbreviations (`*[TERM]: definition`) put a tooltip on every occurrence. A shared glossary file can be added to every page through `pymdownx.snippets` `auto_append`. **[verified]** — [MkDocs Material tooltips](https://squidfunk.github.io/mkdocs-material/reference/tooltips/)
- Navigation: `navigation.instant` (single-page-app loads), `.instant.prefetch` (load on hover), `.instant.progress` (bar after 400 ms), `.instant.preview` (instant previews of header links), `navigation.tracking` (the address bar URL follows the active TOC anchor), `toc.follow` (the sidebar scrolls to keep the active anchor visible), `navigation.top` (back-to-top button that appears when the reader scrolls back up), `navigation.path` (breadcrumbs), `toc.integrate`. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Keyboard: F, S or / opens search. Up/Down select results, Enter opens one, Esc/Tab closes. P/left arrow and N/right arrow go to the previous and next page. Custom keys can be added through `keyboard$`. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Code annotations (numbered markers in code that open a tooltip), content tabs with linked tabs, admonitions, collapsible `???` blocks and Mermaid rendering are standard Material features. **[unverified: prior knowledge]** — see [Material reference](https://squidfunk.github.io/mkdocs-material/reference/)

**mdBook**
- Print: a print icon opens a single page with the whole book (`[output.html.print] enable`, `page-break` between chapters). **[verified]** — [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)
- Search ranking knobs: `limit-results`, `use-boolean-and`, `boost-title`, `boost-hierarchy`, `boost-paragraph`, `expand` (prefix matching), `heading-split-level` (results link to sections). **[verified]** — [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)
- Themes: `default-theme` and `preferred-dark-theme` (default `navy`). There is sidebar folding, a copy button (`copyable`), and edit and repository links. **[verified]** — [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)
- Keyboard: Left/Right arrows move between chapters, S or / opens search, ? opens help, Esc closes help. **[verified]** — [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)

**VitePress**
- Local search is fuzzy full-text search over an in-browser index built with minisearch (`themeConfig.search.provider: 'local'`). Algolia is the alternative. **[verified]** — [VitePress search](https://vitepress.dev/reference/default-theme-search)
- The outline in the aside defaults to level 2 and accepts a range or `'deep'` (h2-h6). `lastUpdated` shows Git timestamps with configurable formatting. `::: code-group` gives tabbed code blocks. **[verified via secondary mirrors only]** — [Mintlify mirror of theme config](https://www.mintlify.com/vuejs/vitepress/customization/theme-config). The official pages were not captured.

**Starlight (Astro)**
- The landing page claims site navigation, search, i18n, "easy-to-read typography", code highlighting, dark/light/auto, and "Fast, accessible". **[verified]** — [Starlight](https://starlight.astro.build/)
- Search is Pagefind. Code is Expressive Code (frames, titles, text markers, diff, copy button). There are asides, synced tabs, steps, file trees, link cards, a right-hand TOC that highlights the current heading, prev/next links and last-updated. **[unverified: prior knowledge]** — the landing page fetched did not list these. See [Starlight docs](https://starlight.astro.build/guides/authoring-content/)

**Pagefind**
- Highlighting: set `highlightParam` on the search side so the query is appended to result URLs, then load `pagefind-highlight.js` on result pages. Pagefind 1.5.0 introduced a "Component UI" that replaces PagefindUI. **[verified]** — [Pagefind highlighting](https://pagefind.app/docs/highlighting/)
- Pagefind loads index chunks and WASM with `fetch()` at run time, so it needs HTTP and does not suit a single `file://` document. **[unverified: prior knowledge]** — not covered by the page fetched.

**Tufte CSS / Gwern.net**
- Tufte CSS sidenotes need no JavaScript: a `<label class="margin-toggle sidenote-number">`, a hidden `<input type="checkbox" class="margin-toggle">` and a `<span class="sidenote">`. Notes sit in the margin on wide screens. On narrow screens they are hidden until the reader taps the toggle. Margin notes use the same markup with a symbol (⊕) instead of a number. **[verified]** — [Tufte CSS](https://edwardtufte.github.io/tufte-css/)
- Tufte CSS also offers `newthought` small-caps openings, epigraphs (`blockquote` with a `footer`), `fullwidth` figures, the ETBook font with Palatino and Georgia fallbacks, and a `#fffff8` background with `#111` text. **[verified]** — [Tufte CSS](https://edwardtufte.github.io/tufte-css/)
- Gwern.net: link popups that can be recursive and can be moved, fullscreened or "stickied", shown as popovers on small screens. Sidenotes use both margins (`sidenotes.js`) and fall back to floating footnotes. Collapsible inline and block sections. Link icons by filetype, domain or topic. Dark mode that classifies which images to invert. A reader mode that removes most UI, including hyperlinks. Lightweight dropcaps and smallcaps. Sortable tables. Lazy client-side transclusion. Click-to-zoom images. Full-width tables and images. JavaScript-free LaTeX math. "JavaScript is *not* required for the core reading experience." **[verified]** — [Gwern.net design](https://gwern.net/design)

**Typst HTML**
- In Typst 0.14 (2025-10-24), HTML export was experimental and behind a feature flag. Typst 0.15 (2026-06-15) adds equations in HTML through MathML and multi-file and bundle export. **[verified]** — [Typst 0.14 changelog](https://typst.app/docs/changelog/0.14.0/), [Typst 0.15 blog](https://typst.app/blog/2026/typst-0.15)

**Not researched this session (gaps, see below):** Docusaurus, Obsidian and Typora export, Markdown Viewer extensions, GitHub rendering, Distill, MyST/Jupyter Book 2, Readwise Reader, Notion export, iA Writer export.

### Inferences
- The strongest reader-facing ideas that work in a single offline file are Quarto's three hover popups (footnotes, citations, cross-references), Quarto and Tufte margin notes, and Gwern's link popups. md2html already has the popup mechanism (glossary hovers), so extending it to footnotes and internal `#links` should cost little.
- Docs-site features that depend on many pages (instant navigation, prefetch, prev/next page) have one-page equivalents: previous/next section, a "jump to next chapter" key, and scrollspy.
- **[unverified: prior knowledge]** Docusaurus has a right-hand TOC with active highlighting, tabs with `groupId` sync, admonitions, and live code blocks. Obsidian's "Export to PDF" and community "Webpage HTML Export" plugin give a self-contained page with graph view and search. GitHub renders Mermaid, math through MathJax, alerts (`> [!NOTE]`) and a generated heading outline. Distill offers hover footnotes and citations, a sticky byline/metadata block, appendix sections, and an L-shaped figure layout grid. Jupyter Book 2 / MyST offers hover previews of cross-references and of linked content across sites. These shape the checklist below but need verification.

### Gaps
- Docusaurus, Obsidian/Typora export, GitHub rendering, Distill, MyST/Jupyter Book 2, Readwise Reader, Notion and iA Writer were not fetched within the tool budget. Everything about them above comes from prior knowledge.
- Whether Quarto `embed-resources` inlines MathJax fonts and the Mermaid runtime was not confirmed this session.
- The VitePress official pages were not captured. Only secondary mirrors were read.

## Specific features: which ones does md2html lack, and which would raise quality most?

### Takeaway
md2html already has most of the "expected" layer (TOC, search, theme, copy buttons, anchors, callouts, zoom). The highest-value gaps are: (1) scrollspy plus the URL following the current section, (2) footnotes as hover popovers or sidenotes, (3) hover previews of internal links and cross-references, (4) a proper print stylesheet, (5) keyboard navigation and back-to-top, (6) highlighting search hits after a jump, with text-fragment deep links, and (7) collapsible sections. Diagrams and math raise the most file-size and offline risk.

### Cited Findings
- Scrollspy: MkDocs Material `toc.follow` keeps the active anchor in view, and `navigation.tracking` writes the active anchor into the URL. Quarto `toc-expand` auto-expands the TOC on scroll. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/), [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Back-to-top: MkDocs Material `navigation.top` shows the button only when the reader scrolls back up. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Reading progress: the only progress bar found is MkDocs Material's page-load bar (`instant.progress`), which is not a reading-position bar. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Hover footnotes, citations and cross-references: Quarto turns all three on by default. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Sidenotes: Tufte CSS does them with CSS only, using a checkbox toggle on narrow screens. Gwern uses JavaScript sidenotes in both margins and falls back to floating footnotes. Quarto offers `reference-location: margin`. **[verified]** — [Tufte CSS](https://edwardtufte.github.io/tufte-css/), [Gwern.net design](https://gwern.net/design), [Quarto article layout](https://quarto.org/docs/authoring/article-layout.html)
- Link previews: Gwern's popups are recursive and can be pinned. MkDocs Material has `navigation.instant.preview`. **[verified]** — [Gwern.net design](https://gwern.net/design), [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Cross-references to figures and tables: Quarto gives anchors and hover previews to cross-referenced figures and tables. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Tabbed content: Quarto `.panel-tabset` with `group` sync and VitePress `::: code-group`. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html), [VitePress mirror](https://www.mintlify.com/vuejs/vitepress/customization/theme-config)
- Collapsible sections: Quarto `code-fold` uses `<details>`. Gwern has inline and block collapse. **[verified]** — [Quarto HTML code](https://quarto.org/docs/output-formats/html-code.html), [Gwern.net design](https://gwern.net/design)
- Math offline: pandoc's default MathML needs no JavaScript. Typst 0.15 emits MathML too. MathJax under `--embed-resources` may lose fonts. Gwern renders LaTeX without JavaScript. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html), [Typst 0.15](https://typst.app/blog/2026/typst-0.15), [Gwern.net design](https://gwern.net/design)
- Syntax highlighting: Quarto uses adaptive themes with light/dark pairs. Pandoc has named styles and an `idiomatic` mode. **[verified]** — [Quarto HTML code](https://quarto.org/docs/output-formats/html-code.html), [Pandoc manual](https://pandoc.org/MANUAL.html)
- Print: mdBook builds a whole-book print page with optional chapter page breaks. Gwern lists print support. **[verified]** — [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html), [Gwern.net design](https://gwern.net/design)
- Embedded images: pandoc and Quarto both inline images as `data:` URIs under `embed-resources`. Pandoc can inline SVG as markup (`inline-svg`) and lets single elements opt out with `data-external="1"`. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html), [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- Keyboard: MkDocs Material (F/S// search, P/N or the arrow keys for previous/next) and mdBook (arrow keys for chapters, S or / for search, ? for help). **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/), [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)
- Search hit highlighting after navigation: Pagefind appends the query parameter and highlights it on the target page. **[verified]** — [Pagefind highlighting](https://pagefind.app/docs/highlighting/)
- Text fragments (`#:~:text=`): Firefox has supported them natively since version 131 (October 2024). Chromium and Safari already supported them. **[verified for Firefox]** — [Mozilla support](https://support.mozilla.org/en-US/kb/text-fragments-firefox/revision/297570), [Bugzilla 1914877](https://bugzilla.mozilla.org/show_bug.cgi?id=1914877). Chromium and Safari support **[unverified: prior knowledge]**.
- Fuzzy search: VitePress local search is fuzzy (minisearch). mdBook offers `expand` (prefix) and boosts by field (title, hierarchy, paragraph). **[verified]** — [VitePress search](https://vitepress.dev/reference/default-theme-search), [mdBook renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)
- Last-updated metadata: VitePress `lastUpdated` reads Git timestamps. **[verified, secondary]** — [VitePress mirror](https://www.mintlify.com/vuejs/vitepress/customization/theme-config)
- Reader mode: Gwern's alternative view removes most UI, including hyperlinks. **[verified]** — [Gwern.net design](https://gwern.net/design)
- Sortable tables and full-width tables and images (Gwern). Full-width figures (Tufte, Quarto `.column-page` and `.column-screen`). **[verified]** — [Gwern.net design](https://gwern.net/design), [Tufte CSS](https://edwardtufte.github.io/tufte-css/), [Quarto article layout](https://quarto.org/docs/authoring/article-layout.html)

### Inferences
Gap matrix against the md2html baseline, ranked by benefit to the reader against cost in a single offline file:

| Rank | Feature | md2html today | Who does it | Offline / size cost | Notes |
|---|---|---|---|---|---|
| 1 | Scrollspy: active section highlighted and kept in view in the outline, optionally written into the URL (`history.replaceState`) | Not listed | MkDocs `toc.follow` and `navigation.tracking`, Quarto, Starlight, Docusaurus, VitePress | A few hundred bytes of JavaScript (IntersectionObserver) | Expected by readers of long documents. Pairs with the existing hideable outline. |
| 2 | Footnotes as hover popovers, and as sidenotes on wide screens | Glossary has hovers. Footnotes are not listed. | Quarto `footnotes-hover` and `reference-location: margin`, Tufte, Gwern, Distill | Reuses the glossary popover code. Sidenotes in CSS only, Tufte style. | Highest typographic payoff. Keep back-links. |
| 3 | Hover previews of internal `#` links and cross-references (figure, table, section) | No | Quarto `crossrefs-hover`, Gwern popups, MyST | Low, because the content is already in the DOM (clone the target node) | Needs numbered figures and tables plus `@fig:` style references first. |
| 4 | Print stylesheet: hide chrome, expand collapsed blocks, print URLs after external links (`a[href^=http]::after{content:" (" attr(href) ")"}`), `break-inside: avoid` on figures, tables and code, `break-after: avoid` on headings, glossary popovers printed inline or as endnotes | Not listed | mdBook print page, Gwern | CSS only | Cheap. Readers turn these documents into PDFs. |
| 5 | Keyboard navigation: j/k or [ ] for next/previous section, t for the outline, ? for a help overlay, / as a second search key | Only Cmd/Ctrl-K | MkDocs Material, mdBook | Tiny | Must ignore keys typed into input fields. |
| 6 | Back-to-top button that appears when the reader scrolls back up | Not listed | MkDocs `navigation.top` | Tiny | |
| 7 | Search hits highlighted on the target with `<mark>`, deep links to results, and "copy link to this passage" producing a `#:~:text=` fragment | Search exists. Highlighting is unknown. | Pagefind `highlightParam`, browser text fragments | Tiny, using the CSS Custom Highlight API or `<mark>` | Native fragments work in every major engine since Firefox 131. |
| 8 | Collapsible sections (`<details>` per heading, or author-marked) and expand/collapse all | Not listed | Quarto `code-fold`, Gwern, MkDocs `???` | None, since `<details>` is native | Print CSS must open them. |
| 9 | Tabbed content with synced groups | No | Quarto `.panel-tabset`, Material, VitePress, Starlight, Docusaurus | Small | Useful for docs, less so for essays. |
| 10 | `--embed-images` option (`data:` URIs or inline SVG, with a per-image opt-out) | Images are linked | pandoc and Quarto `embed-resources` | File-size cost is the user's choice | Makes "single file" true for mail and archiving. Offer it as opt-in. |
| 11 | Math as MathML (no JavaScript, no fonts) | Unknown | pandoc default, Typst 0.15 | Zero runtime | Prefer MathML over embedding KaTeX or MathJax. |
| 12 | Mermaid / diagrams rendered to SVG at build time | Unknown | Material and GitHub use client-side Mermaid | The client-side Mermaid runtime is large (**[unverified: prior knowledge]** about 1 MB or more minified), so render at build time | Build-time SVG keeps the file small and offline. |
| 13 | Reading time and a metadata block (author, date, last updated from Git or mtime, word count) | Not listed | VitePress `lastUpdated`, Distill byline | Build time only | Cheap and orienting. |
| 14 | Code annotations, line numbers, wrap/scroll toggle, code folding | Copy only | Quarto, Material, Expressive Code | Small | Medium value for technical documents. |
| 15 | Reading-position progress bar | No | No surveyed tool ships one as a core feature | Tiny | Nice to have. Scrollspy gives most of the same orientation. |
| 16 | Sortable and responsive tables (horizontal scroll wrapper, sticky header) | Not listed | Gwern sortable tables | Small | |
| 17 | Reader or focus mode (hide outline and chrome, optionally links) | Partial: the outline can be hidden | Gwern reader mode | Tiny | |
| 18 | Link icons for external, PDF and similar links | No | Quarto `link-external-icon`, Gwern link icons | CSS plus a few SVG glyphs | |

### Gaps
- No source measured whether a reading-progress bar improves reading. Its ranking is a judgment.
- md2html's current print CSS and math or Mermaid handling were not inspected in this research. Check the code before treating rows 4, 11 and 12 as gaps.

## How do these tools keep a single file small (CSS/JS budget, fonts)?

### Takeaway
The proven levers are native platform features instead of libraries (MathML, `<details>`, CSS-only sidenotes, system fonts), work done at build time instead of run time (highlighting, diagrams, math), and opt-in embedding of heavy resources. Nothing found gives authoritative byte budgets for these tools.

### Cited Findings
- Pandoc's default math output is MathML: native in browsers, with no script or font payload. MathJax under `--embed-resources` can lose its fonts, because fonts and scripts that JavaScript loads at run time cannot be embedded. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- Pandoc lets single resources opt out of embedding with `data-external="1"`. **[verified]** — [Pandoc manual](https://pandoc.org/MANUAL.html)
- Quarto `minimal: true` removes the bootstrap theme, popups, tabsets and code copy, so its full feature set is a deliberate weight trade-off. **[verified]** — [Quarto HTML basics](https://quarto.org/docs/output-formats/html-basics.html)
- MkDocs Material `navigation.prune` cuts navigation markup from each page's HTML, which shows that navigation markup is a known size cost. **[verified]** — [MkDocs Material navigation](https://squidfunk.github.io/mkdocs-material/setup/setting-up-navigation/)
- Gwern: "uses many tricks to offer nice features like sidenotes or LaTeX math at minimal cost", and JavaScript is not required for the core reading experience. **[verified]** — [Gwern.net design](https://gwern.net/design)
- Tufte CSS loads ETBook as separate .ttf files for regular, bold and italic, with Palatino and Georgia fallbacks. A single-file tool would have to embed that font as base64. **[verified]** — [Tufte CSS](https://edwardtufte.github.io/tufte-css/)

### Inferences
- For md2html, the best font strategy is a system font stack by default (for example `system-ui` for UI, and `ui-serif, Charter, "Iowan Old Style", Georgia, serif` or a Charter-like stack for text), with an optional embedded WOFF2 font subset to the characters used (pyftsubset or a Rust subsetter), base64-encoded. Base64 adds about 33% to the binary size. That last figure is general knowledge, not sourced this session.
- Keep highlighting at build time (syntect or tree-sitter in Rust) with CSS-variable themes, so light and dark swap without a second set of styles.
- Compress the embedded search index (short keys, no stored full text when snippets can be cut from the DOM) and build it lazily from the DOM on first Cmd-K instead of shipping it, if index size matters.

### Gaps
- No sourced CSS/JS byte figures were found for Quarto, Material, Starlight or mdBook output. Measure them directly by building a sample page with each.
- Typst and pandoc font-embedding behavior for `@font-face` inside CSS was not confirmed. The pandoc manual excerpt did not say.

## Typography and readability practices of the best renderers

### Takeaway
Leading renderers use a single readable column, fluid type, generous line height, and progressive-enhancement CSS such as `text-wrap: pretty` and `balance`. Classic typography (small caps, dropcaps, epigraphs, hanging punctuation) is still found mainly in Tufte and Gwern-style essays, not in docs frameworks.

### Cited Findings
- `text-wrap: pretty` is supported in Chrome since 117 and in Safari since 26 (WebKit announced it in STP 216, April 2025). Firefox does not support it yet. Browsers that lack it fall back to normal wrapping, so it is safe as progressive enhancement. **[verified]** — [caniuse text-wrap: pretty](https://caniuse.com/wf-text-wrap-pretty), [WebKit blog](https://webkit.org/?p=16547), [MDN text-wrap-style](https://developer.mozilla.org/docs/Web/CSS/text-wrap-style)
- A February 2026 essay discusses `text-wrap: pretty` together with justified text. **[verified, title only]** — [matklad: Justifying text-wrap: pretty](https://matklad.github.io/2026/02/14/justifying-text-wrap-pretty.html)
- Tufte CSS uses small-caps "newthought" openings, epigraphs, a warm off-white background (`#fffff8`) with near-black text (`#111`), and a serif text face. **[verified]** — [Tufte CSS](https://edwardtufte.github.io/tufte-css/)
- Gwern uses Source Serif/Sans, lightweight dropcaps and small caps, and reader mode. **[verified, partial]** — [Gwern.net design](https://gwern.net/design)
- Starlight advertises "easy-to-read typography" as a headline feature. **[verified]** — [Starlight](https://starlight.astro.build/)

### Inferences
- A recommended CSS baseline for md2html, from established practice and **[unverified: prior knowledge]** of Butterick's *Practical Typography* (45-90 characters per line, line spacing 120-145% of point size) and common renderer CSS:
  - `max-width: 66ch` for body text, a range of 60-75ch, with wide elements (figures, tables, code) allowed to break out of the column.
  - `line-height` about 1.5-1.6 for body text and about 1.15-1.25 for headings.
  - `text-wrap: pretty` on `p, li, figcaption` and `text-wrap: balance` on `h1-h4` and captions.
  - `hyphens: auto` with a correct `lang` attribute, which matters most if text is justified.
  - `font-variant-numeric: tabular-nums` in tables, `oldstyle-nums` optionally in prose, `font-feature-settings`/`font-variant-ligatures: common-ligatures`, `font-kerning: normal`, and `font-optical-sizing: auto` for variable fonts.
  - `hanging-punctuation: first allow-end` (Safari only).
  - A vertical rhythm built from one spacing unit (for example `--space: 1.5rem`) for paragraphs, headings and figures, with headings closer to the text that follows them than to the text before.
  - `scroll-margin-top` on headings so anchor jumps clear any sticky header.
  - `prefers-reduced-motion` to switch off smooth scroll and zoom animations.
  - A focus-visible ring and WCAG AA contrast in both themes.
- md2html already scales text size from 85% to 200%. Scaling by `rem`/`ch` keeps the measure stable as text size changes.

### Gaps
- Butterick's figures and current Firefox status of `hanging-punctuation` were not fetched this session. Verify them at [Practical Typography](https://practicaltypography.com/line-length.html) and [MDN hanging-punctuation](https://developer.mozilla.org/docs/Web/CSS/hanging-punctuation).
- Tufte CSS's exact column widths (the main column is believed to be 55% with a 50% sidenote margin) were not given on the page fetched. Read them from `tufte.css` directly.
