# md2html

Markdown in, one complete web page out. The page brings its own outline, search, light and dark themes, text size, glossary, footnotes, figures that enlarge, coloured code, typeset maths and diagrams, in seventeen languages, and it is a single file that loads nothing from the network: mail it, copy it to another machine, or open it straight from a folder.

![A page made by md2html: the outline on the left; search, language, text size and theme, and saving in the bar.](docs/capabilities/light.jpg)

**Everything md2html does, shown on a page it made: [docs/capabilities.md](docs/capabilities.md)** (render it with `md2html docs/capabilities.md`, in any of its seventeen languages).

## How to read this

- **Chapters 1 to 3** say where it runs, what it does, and get a first page made. Two minutes.
- **Chapters 4 to 7** are for whoever writes the Markdown: what it reads, the conventions that make a page better, the settings, and the shared glossary.
- **Chapter 8** is what the reader of a page gets.
- **Chapters 9 to 12** are the command line, safety and quality, the limits, and where md2html comes from.

## 1. Where it runs

| | Where |
|---|---|
| **The tool** | macOS (Apple silicon and Intel), Linux and Windows: one Rust program with nothing beyond the standard library, under a megabyte, tested on all three on every change |
| **The pages** | current Chrome, Edge, Firefox and Safari, on computers, tablets and phones, online or offline, with screen readers and the keyboard throughout |
| **Opened from** | a web server, a shared folder, an email attachment, or the disk: a page is one file |
| **Print and PDF** | any browser prints a page with its own print layout; `--pdf` makes the PDF itself, with the Chrome, Chromium or Edge already installed |
| **E-readers** | `--epub` makes an EPUB 3 |
| **Languages** | the page's own words in seventeen languages, right to left included |

## 2. What it does

- **Reads Markdown the way CommonMark does** (640 of the specification's 652 examples pass), with GitHub's tables, task lists, strikethrough, autolinks, footnotes and alerts.
- **Writes one page that needs nothing else**: outline, search, light and dark themes, text size, a glossary whose words explain themselves, footnotes that open in place, figures that enlarge.
- **Code, maths and diagrams without a script library**: code coloured in some twenty languages, maths as MathML, Mermaid flowcharts and sequence diagrams drawn as SVG.
- **Sharing**: a link to any chapter, table row, figure, diagram, code block or selected passage, copied with one click.
- **Seventeen languages**, translations linked by a switch that keeps the reader's place, Hebrew and Yiddish right to left, and on-device translation where the browser offers it.
- **Keeping**: the page reads offline; `--pdf` and `--epub` make a PDF with bookmarks and an e-book.
- **Live pages**: `--serve` reloads every open page on save; `--live` lets a published page refresh itself when it changes, keeping the reader's place.
- **Safe for Markdown you did not write**: no script from the text, ever; no file read outside the document's folder; no input that stalls or crashes it ([SECURITY.md](SECURITY.md)).
- **Accessible**: no findings against WCAG 2.2 AA, in both themes and at phone width.
- **Fast and small**: a 4,000-line document, diagrams and maths included, in under a tenth of a second; no dependency beyond the Rust standard library.

## 3. Install and make a first page

You need Rust 1.85 or newer.

```sh
cargo install --git https://github.com/githubrobbi/md2html md2html --locked
```

Then:

```sh
md2html notes.md                 # writes notes.html beside it
md2html notes.md out.html        # a chosen output path
md2html docs/                    # every .md under docs/, each beside itself
md2html docs/ -o site/ --nav     # into site/, linked with previous and next
md2html notes.md --check         # write nothing; say what to fix
md2html notes.md --serve         # show it on this computer; it reloads on save
md2html notes.md --pdf --epub    # a PDF and an EPUB beside the page
cat notes.md | md2html - > notes.html
```

md2html refuses to write over its input or over another Markdown file.

From a checkout, with [`just`](https://just.systems):

```sh
just use-local       # build md2html here and install it to ~/bin
just release-local   # archives for macOS, Linux and Windows, ARM64 and x86-64, in dist/
```

`just release-local` builds every platform from one machine (a Mac builds all six; Linux and Windows build everything but macOS), checks each binary, and writes the SHA-256 of every archive beside them. Nothing is uploaded.

> **Tip:** try it on something real: `md2html README.md`.

## 4. What a document can use

Everything CommonMark has, and what GitHub adds to it:

| You write | You get |
|---|---|
| `# Title` | the page title; the paragraph after it becomes the lead |
| `## Chapter`, `### Section` | chapters and sections, listed in the outline |
| `#### Smaller` to `######` | smaller headings, not in the outline |
| `**bold**`, `*italic*`, `***both***`, `~~struck~~`, `` `code` `` | bold, italic, both, struck, inline code |
| `[text](url "title")`, `[text][ref]`, `<https://…>`, a bare `https://…` | links |
| `- item`, `1. item`, nested by indenting | lists, as deep as you like |
| `- [ ] open`, `- [x] done` | a task list |
| `> quoted text` | a quotation |
| `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`, `> [!CAUTION]` | a callout of that kind, as on GitHub |
| `> **Note:**`, `> **Tip:**`, `> **Warning:**` | the same callouts, the older way |
| a pipe table, `:--`, `:-:` and `--:` in its rule | a table with its columns aligned; a wide one scrolls sideways |
| a fenced code block with a language | code, coloured, with a copy button |
| `$x^2$`, `$$ \frac{a}{b} $$`, a ` ```math ` block | maths, typeset as MathML |
| a ` ```mermaid ` flowchart or sequence diagram | the diagram, drawn as SVG in the page's colours, without a script |
| `text[^1]` and `[^1]: the note` | a footnote: the number explains itself on hover |
| a line that is only `![caption](path)` | a figure with its caption, enlarged over the page on a click |
| `![caption](path){width=50%}` | the same at a chosen width |
| `![alt](icon.svg)` inside a sentence | an image on the line |
| `<kbd>`, `<sub>`, `<sup>`, `<details>`, `<br>` and other harmless tags | those tags |
| `---` on a line of its own | a rule |
| a `---` block of `key: value` lines at the very top | settings for the page (chapter 6) |

Code is coloured for Rust, Python, JavaScript and TypeScript, Go, C and C++, Java, Kotlin, Swift and C#, shell and shell sessions, SQL, Ruby, Lua, PHP, CSS, HTML and XML, JSON, YAML, TOML and INI, Dockerfiles and diffs. Any other language is shown plain.

> **Note:** a construct the renderer does not know is not dropped and not guessed at. It appears verbatim, which is how you notice it. `--check` lists every such place.

## 5. Conventions that make a page better

None of these is required. Each one is ordinary Markdown that also reads well as plain text.

**The lead.** The first paragraph after the `# Title` is set larger, as the opening of the page. Write it as the one paragraph a reader needs if they read nothing else. It is also the page's description for link previews.

**The reading guide.** A chapter called `How to read this` turns its bullets into cards. Start each bullet with a bold phrase that names chapters by number, as this README does, and the card links to the first chapter it names.

**Numbered chapters.** A chapter heading such as `## 3. What a document can use` is what a card's number points at. The numbers are yours; the renderer does not renumber anything.

**The glossary.** A chapter whose title has `Glossary` in it holds the page's words, as a table of term and meaning (a third column, `Also`, names other spellings the prose uses). It is not printed where it stands. Instead:

- the first mention of each term in every chapter becomes a dotted link that explains itself on hover;
- a click jumps to the entry at the foot of the page, and every entry has a way back to the line you were reading;
- the entries are searchable, so searching for a term finds its meaning.

**Several top-level headings.** A document that uses `#` for its chapters gets the first one as its title and the others as chapters.

**Lines that are skipped.** HTML comments and a chapter called `Contents` are left out: the outline is the contents.

## 6. Settings

Front matter at the very top of the Markdown sets the page. The same settings work on the command line, as `--set key=value` or as the flags in chapter 9; the command line wins.

```yaml
---
title: The plans
lang: de
brand: Maplewood ADU
kicker: plans
footer: Working drawings. Schematic, not for construction.
description: The drawings of each version, to compare.
---
```

| Setting | What it does |
|---|---|
| `title` | the page's title, instead of its first heading |
| `lang` | the page's language; the page's own words (search, glossary, callouts, dates) follow it in English, German, Hebrew, Indonesian, Dutch, Javanese, Yiddish, Low German, French, Italian and Spanish; Hebrew and Yiddish are laid out right to left |
| `translated_from`, `source_sha256` | for a translation: its original and the original's fingerprint, so `--check` notices when the original changes |
| `translations` | `false` for no language switch, `device` for one even without translations (the reader's browser translates) |
| `brand`, `brand_link` | the name in the bar instead of md2html, and where it leads |
| `kicker` | the tag beside the name, instead of the file name |
| `eyebrow` | the line above the title, instead of file name and date |
| `date`, `version` | the date shown (`2026-11-21` is written out in the page's language), and a version beside it |
| `description`, `author` | the page's meta description and author |
| `footer` | the fine print at the foot; Markdown allowed |
| `css`, `head` | a stylesheet, or HTML for the head, from a file next to the Markdown |
| `large` | larger type, for a page read at half a screen |
| `embed` | put the images inside the page too, so it is truly one file |
| `toc` | `false` for no outline |
| `mermaid`, `mermaid_js` | how diagrams are drawn (`native`, `js` or `code`), and a local mermaid.js for the kinds md2html does not draw |
| `math` | `false` to read `$` as a dollar sign |
| `glossary` | `false` to leave the shared glossary out |
| `html` | `true` for rich raw HTML (images, links, video, SVG, layout), never anything that runs (chapter 10) |
| `csp` | `true` for a Content-Security-Policy (chapter 10) |
| `index` | `true` to let search engines index the page; pages say `noindex` by default |

`SOURCE_DATE_EPOCH` fixes the date, so the same Markdown renders to the same bytes.

### Languages

A translation is another Markdown file beside the original, named for its language: `notes.de.md` is the German `notes.md`. Render them together (`md2html notes.md notes.*.md`) and each page gets a language switch in the bar, opening at the same chapter in the other language. The translation's front matter names its original and the original's fingerprint, which `--check` prints when it is missing or out of date:

```yaml
---
translated_from: notes.md
source_sha256: 5b0c...e1
---
```

## 7. The shared glossary

Words that come up on many pages can be explained once.

Put a Markdown page with a `Glossary` chapter at `~/.config/md2html/glossary.md`, or name one with the environment variable `MD2HTML_GLOSSARY`. From then on its terms link on every page you render, exactly like the page's own.

- A page lists only the shared terms its prose actually uses.
- Each of those has an **all terms** link to the full list. That is the shared page's twin, so render the shared page itself once. To point somewhere else, set `MD2HTML_GLOSSARY_URL`.
- A term the page defines itself wins over the shared one.
- `--no-glossary` renders a page without it.

## 8. On the page

What the reader of a rendered page can do:

| To | Do this |
|---|---|
| Find something | **Search** in the bar, Command (Control) with K, or `/`; the words found are lit where you land |
| See where you are | the outline marks the chapter you are reading, and the bar names it |
| Hide the outline | the switch at the left of the bar, or Command (Control) with backslash |
| Bring it back for a moment | rest the pointer on the rail at the left edge, or click it: the outline opens as a drawer |
| Make the text larger or smaller | **Aa** in the bar, then **−** and **+**, from 85 to 200 percent; the number between them returns to 100. Or Command (Control) with + and − |
| Change the theme | **Aa**, then System, Light or Dark |
| Read it in another language | the globe in the bar, where the page has translations; the page opens at the same chapter |
| Keep it | the download button at the end of the bar, *Save or print*: the page itself (it reads offline when its images are inside it, `embed`), print or save as PDF, and the PDF and EPUB when the author made them |
| Enlarge a figure | click it: it fills the window. Two more clicks bring it closer, and the image follows the pointer. The next click, or Escape, puts it back |
| Read a footnote | rest on its number; click to go to it, and **back** returns |
| See what a dotted word means | rest on it; click to go to its entry |
| Copy a code block | **copy** on the block |
| Share a section | the share symbol to the right of a heading copies its link (on a phone, the share sheet opens) |
| Share a table row | the share symbol at the end of the row under the pointer; the link opens the page on that row, highlighted |
| Share a figure, a diagram, a code block | the share symbol in its corner while the pointer is on it |
| Share a passage | select the words: **Link to this passage** copies a link that opens the page on exactly them |
| Print it | the controls drop away, links print their addresses, nothing is cut across a page, and each page carries the title and its number |

Every page ends with one line, *Made with md2html* and its version, linked to md2html's home.

The outline, the text size and the theme are remembered by the browser, per reader. Every control works from the keyboard, and the page follows the system's reduced-motion and high-contrast settings.

## 9. The command line

```text
md2html [options] <input>...      a file, a folder, or - for stdin
md2html <in.md> <out.html>

  -o, --out <path>   the page, a folder for several, or - for stdout
  --title, --lang, --brand, --footer <text>
  --css <file>, --head <file>, --set key=value
  --large, --embed, --no-glossary, --no-toc, --no-math, --index
  --html, --csp
  --mermaid native|js|code, --mermaid-js <file>
  --nav              link the pages of one run with previous and next
  --fragment         only the HTML of the Markdown, no page around it
  --pdf              also a PDF beside the page, printed by Chrome
  --epub             also an EPUB beside the page, for e-readers
  --max-input <MB>   the largest Markdown file read (64)
  --max-embed <MB>   the largest image embedded (32)
  --check            write nothing; list what to fix; exit 1 if anything
  --watch            render again whenever an input changes
  --serve [--port n] serve the pages on this computer; they reload on every save
  --live, --ttl <s>  the page has a time to live: then it reloads if it changed
  -q, --quiet
```

A link to another Markdown file that exists beside the page, `[setup](setup.md#install)`, is written as a link to its page, `setup.html#install`, so a folder of Markdown becomes a folder of linked pages.

`--pdf` prints the page with the Chrome (or Chromium, or Edge) already on the computer, found where it usually is or named by `MD2HTML_CHROME`: the page's own print layout, its headings as the PDF's bookmarks, the title at the head of each page and the page number at the foot. Chrome runs headless with a fresh, throwaway profile and a time limit. Without a Chrome, md2html says so and the page is still written. `--epub` makes an EPUB 3 from the same page, written by md2html itself: the article as XHTML, the outline as its contents, the images packed inside, diagrams and maths kept, footnotes marked so that e-readers open them where they are mentioned, right to left where the language is. `pdf: true` and `epub: true` in the front matter do the same. The page's download button then offers both.

Pages can keep themselves current, and keep the reader's place when they do. `md2html notes.md --serve` serves the pages on this computer (127.0.0.1 only, the document's folder only) and every open page reloads the moment the Markdown is saved. `--live` (or `live: true`, or `live: 30` for thirty seconds) gives a published page its own time to live, two minutes by default: when it runs out, and when the reader returns to the tab, the page asks the server it came from for the first kilobyte of itself, where its fingerprint is, and reloads only if that changed; a page opened from a file simply reloads, which goes well with `--watch`; offline, nothing happens. After a reload the reader is where they were: the paragraph at the top of the screen is found again by its text in its section, then anywhere, then by the closest text or its position in the section; if the section is gone, the one before it; if nothing is left, the top. A reload waits while the reader is selecting text or has a menu, search or a figure open.

Every render reports what an author should look at, with the line: a link to an anchor that is not on the page, a file that is not there, a footnote without its note, raw HTML shown as text, an unknown setting. `--check` does only that and exits 1 when there is anything, which makes it a check for a build or an agent's last step.

## 10. Safety and quality

A page can be rendered from Markdown you did not write.

md2html is built so that it cannot be used to make a page that does harm: whatever the Markdown holds, the page runs only md2html's own code. [SECURITY.md](SECURITY.md) states the guarantees and how to report a problem.

- **No script from the text, ever.** Raw HTML is shown as text, except a short list of harmless tags (`<kbd>`, `<sub>`, `<details>` and the like) without any attribute that could run code. `--html` lets rich HTML through (images, links, video, SVG, tables, layout) but never a script, an event handler, a frame, a plug-in, a form, a redirect or a style that fetches; the author's `head` file keeps only `meta`, `link` and `style`.
- **No script links.** A `javascript:`, `vbscript:` or `data:` link is not written, however it is spelled or escaped; only its text remains. The same goes for the brand's link in the front matter.
- **A document reads nothing outside its folder.** Files the front matter names (`css`, `head`, `mermaid_js`, the original of a translation) must lie in the document's folder and be of their kind; an embedded image must lie in the document's folder or the one md2html runs in, and its bytes must be an image, whatever its name says. The command line may name any file.
- **Bounded time, bounded size.** Every part of the reader is linear in its input, and nesting is capped: a hundred thousand asterisks, brackets or quote marks, tens of thousands of footnotes or link definitions render in a fraction of a second. A Markdown file larger than 64 MB, or an image larger than 32 MB to embed, is refused; `--max-input` and `--max-embed` raise those on the command line, never from a document.
- **PDFs printed offline.** `--pdf` runs Chrome headless with a throwaway profile, a time limit and no network.
- **A Content-Security-Policy, when asked.** `--csp` adds a policy that lets exactly the page's own scripts and styles run, by their hashes, and forbids every connection out.

The tests render whole pages and read them back: structure, security, the settings, the glossary, footnotes, the build scripts that post-process md2html's pages, and the CommonMark specification's 652 examples, of which the count passing may rise but never fall (`cargo test`).

md2html has no third-party code: the binary is the standard library and md2html. Its toolchain is pinned (`rust-toolchain.toml`), `cargo deny` and `cargo vet` gate any crate that would ever come in, and the test tools and CI actions are pinned and vetted as [the supply-chain page](docs/SUPPLY-CHAIN.md) records.

The pages are also checked in Chrome (`tests/browser`): an accessibility audit with axe-core against WCAG 2.2 AA, in both themes and at phone width, finds nothing; and a click-through uses every control the way a reader would, by mouse and by keyboard, with no script error. The CI runs all of it on every push, on macOS and Ubuntu and on the oldest supported Rust, with clippy's pedantic lints as errors.

## 11. Limits

- **Twelve CommonMark examples fail**, all corner cases: tabs inside list markers, link definitions spread over several lines, and named entities beyond the common ones (those are left for the browser to show).
- **Two deliberate departures** for pages, to keep what authors mean: `---` straight under a paragraph is a rule unless the paragraph is a single heading-like line, and a numbered list may follow a line that ends in a colon whatever its first number. `--fragment` and the tests read strictly.
- **Maths is a subset of TeX**: the everyday commands, Greek, accents, matrices and cases. A formula it cannot read is shown as its TeX.
- **Diagrams: flowcharts and sequence diagrams.** md2html lays them out itself. Other Mermaid kinds need `--mermaid-js FILE`, a local copy of mermaid.js that then goes into that page (about 2.7 MB); without it they stay as code.
- **Images are linked, not embedded**, unless you ask with `--embed`. Copy them along otherwise.
- **The page's own words** are in seventeen languages: English, German, Hebrew, Indonesian, Dutch, Javanese, Yiddish, Low German, French, Italian, Spanish, Japanese (in kanji and kana, `ja`; in kana alone, `ja-Hira`; in rōmaji, `ja-Latn`), Mandarin Chinese (simplified, `zh-Hans`; traditional, `zh-Hant`) and Cantonese (`yue`). Another language gets English words, and its page can still be translated on the reader's device where the browser can.

## 12. Where it comes from

md2html began as the renderer of one project's handbook and was made general so that any Markdown page could use it; version 0.4 replaced its handbook subset with a full CommonMark reader. The changes, version by version, are in [CHANGELOG.md](CHANGELOG.md).

Licensed MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`. The CommonMark examples in `tests/commonmark` are the specification's, under CC BY-SA 4.0.

## Glossary

| Term | Meaning | Also |
|---|---|---|
| self-contained | Everything the page needs is inside the one HTML file: styles, scripts, the search index. Only an image file is outside it, unless embedded. | |
| outline | The list of chapters and sections at the left of the page, made from the headings. | |
| drawer | The outline sliding out over the page while it is hidden, and closing again when you are done with it. | |
| lead | The first paragraph after the title, set larger as the opening of the page. | |
| reading guide | The chapter called "How to read this", whose bullets become cards that link to chapters. | |
| callout | A boxed aside: a note, a tip, something important, a warning or a caution. | callouts |
| figure | An image with its caption, from a line that holds nothing else. | figures |
| front matter | The block of settings between two `---` lines at the very top of a Markdown file. | |
| shared glossary | One glossary page whose terms link on every page you render. | |
| twin | The HTML page rendered from a Markdown file, beside it and with the same name. | |
| CommonMark | The specification of Markdown that most tools now follow, with a test for every rule. | |
