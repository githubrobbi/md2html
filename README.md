# md2html

One Markdown file in, one self-contained HTML page out. The page has an outline, search, a light and a dark theme, text you can make larger, and a glossary that explains its own words. It is a single file that loads nothing from the network, so it can be mailed, copied to another machine or opened straight from a folder.

## How to read this

- **Chapters 1 and 2** say what it is and get a first page made. Two minutes.
- **Chapters 3 to 5** are for whoever writes the Markdown: what it takes, the conventions that make a page better, and the shared glossary.
- **Chapter 6** is what the reader of a page gets.
- **Chapters 7 and 8** are the limits, and where the tool comes from.

## 1. What it is

A small command-line tool. You give it a Markdown file; it writes an HTML page beside it.

![This README, rendered by md2html: the outline on the left, the bar with search, text size and theme on top.](docs/page.png)

It is written in Rust with the standard library only. There is no Markdown crate inside and nothing to vendor, so it builds anywhere a Rust toolchain does.

It is not a general Markdown converter. It renders a deliberate subset well, and leaves anything it does not know on the page exactly as written, where you can see it.

## 2. Install and make a first page

You need Rust 1.85 or newer.

```sh
cargo install --git https://github.com/githubrobbi/md2html
```

Then:

```sh
md2html notes.md               # writes notes.html beside it
md2html notes.md out.html      # a chosen output path
md2html card.md --large        # larger type, for a page read at half a screen
md2html notes.md --no-glossary # without the shared glossary (chapter 5)
md2html --version
```

One page per run. The second path is the output, and md2html refuses to write over the input or over another Markdown file.

> **Tip:** to try it on something real, render this file: `md2html README.md`.

## 3. What a document can use

| You write | You get |
|---|---|
| `# Title` | the page title; the paragraph after it becomes the lead |
| `## Chapter` | a chapter, listed in the outline |
| `### Section` | a section, listed under its chapter |
| a paragraph | a paragraph |
| `- item` and `1. item` | bulleted and numbered lists |
| a pipe table with a header row | a table; a wide one scrolls sideways by itself |
| a fenced code block | code with a copy button, and the language named if you gave one |
| `**bold**`, `*italic*`, `` `code` `` | bold, italic, inline code |
| `[text](url)` | a link |
| a line that is only `![caption](path)` | a figure: the image with its caption below, and a click to enlarge it over the page |
| `> Note:`, `> Tip:` or `> Warning:` | a callout of that kind |
| `---` | a rule |

> **Note:** a construct the renderer does not know is not dropped and not guessed at. It appears verbatim, which is how you notice it.

## 4. Conventions that make a page better

None of these is required. Each one is ordinary Markdown that also reads well as plain text.

**The lead.** The first paragraph after the `# Title` is set larger, as the opening of the page. Write it as the one paragraph a reader needs if they read nothing else.

**The reading guide.** A chapter called `How to read this` turns its bullets into cards. Start each bullet with a bold phrase that names chapters by number, as this README does, and the card links to the first chapter it names.

**Numbered chapters.** A chapter heading such as `## 3. What a document can use` is what a card's number points at. The numbers are yours; the renderer does not renumber anything.

**The glossary.** A chapter with `Glossary` in its title holds the page's words, as a table of term and meaning. It is not printed where it stands. Instead:

- the first mention of each term in every chapter becomes a dotted link that explains itself on hover;
- a click jumps to the entry at the foot of the page, and every entry has a way back to the line you were reading;
- the entries are searchable, so searching for a term finds its meaning.

**Lines that are skipped.** HTML comment lines and a chapter called `Contents` are left out: the outline is the contents.

## 5. The shared glossary

Words that come up on many pages can be explained once.

Put a Markdown page with a `Glossary` chapter at `~/.config/md2html/glossary.md`, or name one with the environment variable `MD2HTML_GLOSSARY`. From then on its terms link on every page you render, exactly like the page's own.

- A page lists only the shared terms its prose actually uses.
- Each of those has an **all terms** link to the full list. That is the shared page's twin, so render the shared page itself once. To point somewhere else, set `MD2HTML_GLOSSARY_URL`.
- A term the page defines itself wins over the shared one.
- `--no-glossary` renders a page without it.

## 6. On the page

What the reader of a rendered page can do:

| To | Do this |
|---|---|
| Find something | **Search** in the bar, or Command (Control) with K |
| Hide the outline | the switch at the left of the bar, or Command (Control) with backslash |
| Bring it back for a moment | rest the pointer on the rail at the left edge, or click it: the outline opens as a drawer |
| Make the text larger or smaller | **−** and **+** in the bar, from 85 to 200 percent; the number between them returns to 100 |
| Change the theme | the button at the right of the bar: light, dark, or whatever the system uses |
| Enlarge a figure | click it: it fills the window. Two more clicks bring it closer, and the image follows the pointer, so point at what you want to see. The next click, or Escape, puts it back, and you are where you were |
| Copy a code block | **copy** on the block |
| See what a dotted word means | rest on it; click to go to its entry |
| Link to a section | the **#** that appears beside a heading |

The outline, the text size and the theme are remembered by the browser, per reader.

## 7. Limits

- **Lists are flat.** An indented item under another becomes one more item of the same list.
- **Images are figures only.** An image inside a sentence is not rendered; give it a line of its own.
- **A figure's file travels with the page.** Everything else is inside the one HTML file, but an image is linked, not embedded. Copy the image along, at the same relative path.
- **No block quotes inside list items, and no bold-italic.**
- **Bare brackets.** A mark in square brackets that is not a link, such as a bracketed letter, is taken for the start of a link. Until that is fixed, write such marks another way.
- **Few tests.** A handful, on the search index. The rest is checked by rendering real documents.

## 8. Where it comes from

md2html began as the renderer of the docenta handbook, was carried over to render the handbook of iv, and was then made general so that any Markdown page could use it. All three have the same owner.

That history is why it is opinionated: it was built to make a handbook pleasant to read, not to implement Markdown. When a document needs a construct it lacks, the answer is a change to the renderer.

Licensed MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.

## Glossary

| Term | Meaning |
|---|---|
| self-contained | Everything the page needs is inside the one HTML file: styles, scripts, the search index. Only a figure's image file is outside it. |
| outline | The list of chapters and sections at the left of the page, made from the headings. |
| drawer | The outline sliding out over the page while it is hidden, and closing again when you are done with it. |
| lead | The first paragraph after the title, set larger as the opening of the page. |
| reading guide | The chapter called "How to read this", whose bullets become cards that link to chapters. |
| callout | A boxed aside: a note, a tip or a warning. |
| figure | An image with its caption, from a line that holds nothing else. |
| shared glossary | One glossary page whose terms link on every page you render. |
| twin | The HTML page rendered from a Markdown file, beside it and with the same name. |
