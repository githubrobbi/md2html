# md2html

One Markdown file in, one self-contained HTML page out.

The page carries its own styles and scripts: a sidebar of contents, search, a
light and dark theme, text-size buttons and a live glossary. It loads nothing
from the network, so the single `.html` file can be mailed, copied to another
machine or opened from a folder.

Standard library only. No Markdown crate, nothing to vendor: it builds
anywhere a Rust toolchain does (1.85 or newer, for edition 2024).

## Install

```sh
git clone https://github.com/githubrobbi/md2html
cd md2html
cargo install --path .
```

Or build without installing: `cargo build --release`, and the binary is
`target/release/md2html`.

## Use

```sh
md2html notes.md               # writes notes.html beside it
md2html notes.md out.html      # a chosen output path
md2html card.md --large        # larger type, for a page read at half a screen
```

## What a document can use

A deliberate subset of Markdown:

| Construct | Notes |
|---|---|
| Headings | `#` is the page title; `##` are the chapters in the sidebar |
| Paragraphs | |
| Tables | pipe tables with a header row |
| Fenced code | |
| Lists | bulleted and numbered |
| Rules | |
| Links | `[text](url)` |
| Bold, italic, inline code | |

A construct the renderer does not know appears verbatim in the page, which is
visible rather than silent.

## Conventions

- **The lead.** The `# Title` and the first paragraph after it open the page.
- **How to read this.** In a chapter with that name, bullets that begin with a
  bold number become cards linking to the numbered chapters.
- **Glossary.** A chapter called `Glossary` holding a table of term and meaning
  becomes live: the first mention of each term in every chapter is a dotted
  link that explains itself on hover and jumps to the entry.

## Known gaps

- Block quotes inside list items.
- Bold-italic.
- A bare bracketed mark such as `[V]` is read as the start of a link. Until
  that is fixed, write such marks another way or swap them before rendering.

## Where it comes from

Forked from the handbook renderer of `iv`, which came from the one in
`docenta` (same owner). It is the renderer of those handbooks made general,
not a full Markdown implementation: a construct a document needs and the
renderer lacks means a renderer change.

## License

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
