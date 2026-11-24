<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# The command line

md2html is a program, not a library: what it promises is how it is run, the settings a document gives it, how it ends, and what it writes. This page is that promise, in one place, for the people who run md2html and the scripts they write around it. A test holds it to the code (`crates/md2html/src/cli/contract_tests.rs`): every option the command line reads, every setting, every exit code and every environment variable md2html reads is on this page, and nothing is on it that md2html does not read; `md2html --help` shows the same options, each with its other spellings on one line.

## Running it

```text
md2html [options] <input>...
md2html <in.md> <out.html>
```

An input is a Markdown file, a folder, or `-` for standard input. A folder means every Markdown file under it (`.md`, `.markdown`, `.mdown`, `.mkd`), its subfolders included, except those whose names start with a dot and folders called `target` or `node_modules`; an `index` or `README` comes first in its folder.

Without `-o`, each page is written beside its Markdown with the extension `.html`, and the page of standard input goes to standard output. `md2html <in.md> <out.html>` is the older form of `md2html <in.md> -o <out.html>`: a second input that is not Markdown, not a folder and not `-` is the page. md2html refuses to write a page over its own Markdown or into a Markdown file.

An option's value follows it, or follows `=` in a long option (`--title=Notes`). Options and inputs may come in any order; after `--` every argument is an input. The command line wins over the front matter.

## Options

### Where the pages go

| Option | Takes | What it does |
| --- | --- | --- |
| `-o`, `--out`, `--output` | a path | The page, for one input; a folder for several (or a path that exists as a folder, or ends in `/`), where each page keeps its place under the folder it was found in; `-` for standard output |

### The page

| Option | Takes | What it does |
| --- | --- | --- |
| `--title` | a text | The title, instead of the first heading |
| `--lang` | a language code | The page's language (setting `lang`); without it, the one the file's name gives (`notes.de.md`), else English. The page's own words are in 17 languages: `en`, `de`, `he`, `id`, `nl`, `jv`, `yi`, `nds`, `fr`, `it`, `es`, `ja`, `ja-Hira`, `ja-Latn`, `zh-Hans`, `zh-Hant`, `yue`; another code gets English words |
| `--brand` | a text | The name in the bar, instead of md2html |
| `--footer` | a text | The fine print at the foot, in Markdown |
| `--css` | a file | Adds a stylesheet; repeatable |
| `--head` | a file | Adds the file's HTML to the head; repeatable |
| `--set` | `key=value` | Any setting of the front matter; repeatable |
| `--large` | | Larger type, for a page read at half a screen |
| `--embed`, `--self-contained` | | Puts the images inside the page as well |
| `--no-glossary` | | Leaves the shared glossary out |
| `--no-toc` | | No outline |
| `--html`, `--unsafe-html` | | Trusts raw HTML in the Markdown: rich, never anything that runs; shown as text otherwise |
| `--csp` | | A Content-Security-Policy: only the page's own code runs |
| `--index` | | Lets search engines index the page (it says `noindex` otherwise) |
| `--no-math` | | Reads `$` as a dollar sign, not maths |
| `--mermaid` | `native`, `js` or `code` | How diagrams are drawn: by md2html (the default), by mermaid.js in the browser, or left as code |
| `--mermaid-js` | a file | A local mermaid.js, for the diagram kinds drawn in the browser |
| `--nav` | | Links the pages of one run with previous and next |
| `--fragment` | | Only the HTML of the Markdown, no page around it |
| `--pdf` | | Also a PDF beside the page, printed by Chrome |
| `--epub` | | Also an EPUB beside the page, for e-readers |
| `--live` | | The page has a time to live of two minutes: then it reloads if it changed, keeping the reader's place |
| `--ttl` | seconds, `yes` or `no` | `--live` with this time to live (5 seconds to a day) |
| `--max-input` | megabytes | The largest Markdown file read: 64 unless given, from 1 to 1,048,576 |
| `--max-embed` | megabytes | The largest image put inside a page: 32 unless given, from 1 to 1,048,576 |

### The run

| Option | Takes | What it does |
| --- | --- | --- |
| `--check` | | Writes nothing; reports what to fix, and ends with 1 when there is anything |
| `--watch` | | Renders again whenever an input changes, or a `--css` or `--head` file, or the shared glossary |
| `--serve` | | Serves the pages on this computer (127.0.0.1, the inputs' folder only); every open page reloads when its Markdown is saved |
| `--port` | a port number | The port `--serve` tries first (8000 unless given; the next nineteen after it are tried too); implies `--serve` |
| `-q`, `--quiet` | | No summary and no notes; no warnings either, except with `--check`, where they are the result |
| `-h`, `--help` | | The help, and nothing else |
| `-V`, `--version` | | The version, and nothing else |

## Settings

Front matter is a block of `key: value` lines at the very top of the Markdown, between two lines of `---` (the second may be `...`). A list under a key (`- item` lines) is one value, its items separated by commas; an indented line continues the value above it. The same settings are given on the command line with `--set key=value` and the options above, and the command line wins.

A key is read without regard to case, and `-` and `_` are the same (`Brand-Link` is `brand_link`). A yes or no setting is yes for `true`, `yes`, `on` or `1`, and no for anything else. A key md2html does not know is reported as an unknown setting.

| Setting | Takes | What it does |
| --- | --- | --- |
| `title` | a text | The page's title, instead of its first heading |
| `lang`, `language` | a language code | The page's language: its `lang`, the page's own words, how dates are written, and which way the lines run |
| `brand`, `site` | a text | The name in the bar, instead of md2html |
| `brand_link`, `brand_href`, `home` | an address | Where the name in the bar leads |
| `kicker` | a text | The tag beside the name, instead of the file name |
| `eyebrow` | a text | The line above the title, instead of the file name and the date |
| `date` | a date | The date shown, instead of today; `2026-11-21` is written out in the page's language |
| `version` | a text | A version shown beside the date |
| `description`, `summary` | a text | The page's meta description, instead of its lead |
| `author`, `authors` | a text | The page's author, for its meta tags and the EPUB |
| `footer` | Markdown | The fine print at the foot |
| `css`, `stylesheet` | files | Stylesheets to add, next to the Markdown, separated by commas |
| `head` | files | HTML to add to the head, from files next to the Markdown, separated by commas |
| `large` | yes or no | Larger type |
| `glossary`, `shared_glossary` | yes or no | `no` leaves the shared glossary out |
| `html`, `raw_html` | yes or no | Trusts raw HTML in the Markdown |
| `embed`, `embed_images`, `self_contained` | yes or no | Puts the images inside the page |
| `csp` | yes or no | A Content-Security-Policy |
| `index` | yes or no | Lets search engines index the page |
| `noindex` | yes or no | The opposite of `index` |
| `toc`, `outline` | yes or no | `no` for no outline |
| `math` | yes or no | `no` reads `$` as a dollar sign |
| `mermaid`, `diagrams` | `native`, `js` or `code` | How diagrams are drawn |
| `mermaid_js` | a file | A local mermaid.js |
| `pdf` | yes or no | Also a PDF beside the page |
| `epub` | yes or no | Also an EPUB beside the page |
| `live` | yes, no, or seconds | The page's time to live: yes is two minutes, a number is seconds (`30` or `30s`), held between 5 seconds and a day |
| `translated_from`, `translation_of` | a file | For a translation: the file it translates |
| `source_sha256` | a fingerprint | For a translation: the SHA-256 of its original when it was translated; `--check` reports when it is missing or the original has changed |
| `translations`, `language_switch` | `false` or `device` | `false` for no language switch, `device` for one even without translations (the reader's browser translates) |

Front matter that other tools use and md2html has no use for is passed over without a warning:

| Setting | |
| --- | --- |
| `tags`, `categories`, `keywords`, `layout`, `slug`, `permalink` | Passed over |
| `draft`, `status`, `created`, `updated`, `translated_by`, `translator` | Passed over |

Two limits are set only on the command line, never by a document: `--max-input` and `--max-embed`. A document cannot lift its own limits.

## Exit codes

The numbers are those of `sysexits.h`, besides 0 and 1.

| Code | Name | When |
| --- | --- | --- |
| `0` | done | Everything asked for was done; with `--check`, nothing to fix |
| `1` | findings | `--check` found something to fix |
| `64` | usage | The command line was wrong (an unknown option, a value an option cannot take, no input), or it asked for what md2html refuses: a page over its own Markdown or into a Markdown file, several pages to standard output, `--watch` or `--serve` on standard input |
| `66` | no input | An input is not there, cannot be read, is too large or is not UTF-8, a folder holds no Markdown, or a folder under an input cannot be read (then no page is made); `--watch` and `--serve` end with it when every input has been gone for a while |
| `69` | unavailable | No port was free for `--serve`; the message ends with the last port tried and why it was refused |
| `70` | software | A PDF could not be printed, or printing left something behind; the server stopped unexpectedly |
| `73` | can't create | A page, an EPUB, or a folder for them could not be written, standard output included |
| `74` | I/O error | The server could not start or its listener failed, or a message could not be written (standard output closed, for example) in a run that otherwise succeeded |

A run that meets several problems goes on with the next page and ends with the code of the last one.

## Environment

| Variable | What it does |
| --- | --- |
| `SOURCE_DATE_EPOCH` | Seconds since 1970: the date a page shows and an EPUB records, instead of now, so the same Markdown renders to the same bytes. Empty is not set; anything but a whole number is a warning on each page, which is then made at the time of the run |
| `MD2HTML_GLOSSARY` | The shared glossary, a Markdown file with a `Glossary` chapter. None there is no shared glossary; one there that cannot be read is a warning on each page |
| `HOME`, `USERPROFILE` | The home folder: `HOME`, else `USERPROFILE` (Windows sets that one). Without `MD2HTML_GLOSSARY`, the shared glossary is `.config/md2html/glossary.md` in it |
| `MD2HTML_GLOSSARY_URL` | Where a page's link to the full shared glossary leads, instead of the shared page's own HTML beside it; a value that is not valid Unicode is a warning, and ignored |
| `MD2HTML_CHROME` | The Chrome (or Chromium, or Edge) `--pdf` prints with, instead of the usual places |
| `PATH` | Where `--pdf` looks for Chrome after the usual places |

## What md2html writes

- **Files.** The page, where `-o` says or beside its Markdown as `.html`; with `--epub` and `--pdf`, the book and the PDF beside the page, with its name and `.epub` or `.pdf`. A page is written whole or not at all: through a new temporary file, renamed over the old one. `--check` writes nothing. A file whose name is not valid Unicode (Linux allows any bytes) is read and written under its own name, but no link can name it: its page offers no download of itself and no language versions, other pages' `--nav` does not link it, `--serve` does not serve it, and the page says so in a warning.
- **Standard output.** The page, when it goes there (`-o -`, or standard input without `-o`), and nothing else; otherwise one summary line for each file written (`md2html: notes.md -> notes.html (...)`), `md2html: nothing to fix` after a clean `--check`, the help and the version.
- **Standard error.** What the author should look at, one line each: `<input>:<line>: <what>`, or `<input>: <what>` for the whole page; notes (`md2html: 3 to look at`, what is being watched or served); and errors, `md2html: <what went wrong>`.

## What is stable

What this page names is md2html's contract: the options and their spellings, the settings and their spellings, the exit codes and when each is given, the environment variables, where the files go, and which stream carries what, including the form of a warning line (`<input>:<line>: <what>`). A change to any of them is a change to this page, made in the same pull request, and named in `CHANGELOG.md`.

Not part of the contract: the wording of messages and of the help, the summary line's details, and the inside of a page (its markup, classes, styles and scripts), which may change from one version to the next. Within a version, the same inputs, command line and environment give the same bytes.
