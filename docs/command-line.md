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

An input is a Markdown file, a folder, or `-` for standard input. A folder means every Markdown file under it, its subfolders included, except those whose names start with a dot and folders called `target` or `node_modules`; an `index` or `README` comes first in its folder. A Markdown file is one whose extension, in any case, is `.md`, `.markdown`, `.mdown`, `.mdwn`, `.mkd`, `.mkdn`, `.mkdown`, `.mdtext`, `.mdtxt`, or one of a dialect written in Markdown: `.mdx`, `.qmd`, `.rmd`, `.livemd`. The same list decides everywhere md2html tells Markdown by its name: the files of a folder, the outputs it refuses, a link that is sent to its page, a translation's original.

Without `-o`, each page is written beside its Markdown with the extension `.html`, and the page of standard input goes to standard output. `md2html <in.md> <out.html>` is the older form of `md2html <in.md> -o <out.html>`: a second input that is not Markdown, not a folder and not `-` is the page. md2html refuses to write a page over its own Markdown or into a Markdown file, and two inputs whose pages would be one file (`notes.md` and `notes.qmd`).

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
| `--html`, `--unsafe-html` | | Trusts raw HTML in the Markdown: rich, never anything that runs, and an attribute only where HTML defines it (each other kind named in a warning); shown as text otherwise |
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
| `--root` | a folder | A folder every document of the run may read files from, besides its own folder and below ("What a document may read", below). Given twice, the later is used; one that is not there, or is not a folder, ends the run with 64 |

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

Front matter is a block of `key: value` lines at the very top of the Markdown, between two lines of `---` (the second may be `...`). A list under a key (`- item` lines) is one value, its items separated by commas; an indented line continues the value above it; a blank line, or one that starts with `#`, is passed over. The block is front matter when its first key comes within its first 400 lines; otherwise the opening `---` is a rule. The same settings are given on the command line with `--set key=value` and the options above, and the command line wins.

A key is read without regard to case, and `-` and `_` are the same (`Brand-Link` is `brand_link`). A yes or no setting is yes for `true`, `yes`, `on` or `1`, and no for anything else (`live` reads its own values, below). A key md2html does not know is reported as an unknown setting. A setting given twice, by one key or by two of its names (`lang` and `language`), is reported with the line it is given again on, and the one given later in the block is used.

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
| `mermaid_js` | a file | Refused: mermaid.js runs in the page, so only `--mermaid-js` gives it |
| `root` | a folder | Refused: it would widen the folders a document reads from, so only `--root` gives one |
| `pdf` | yes or no | Also a PDF beside the page |
| `epub` | yes or no | Also an EPUB beside the page |
| `live` | yes (`true`, `yes`, `on`), no (`false`, `no`, `off`), or seconds | The page's time to live: yes is two minutes, a number is seconds (`30` or `30s`), held between 5 seconds and a day (so `1` is 5 seconds, and `0` is not no); any other value is an error |
| `translated_from`, `translation_of` | a file | For a translation: the file it translates |
| `source_sha256` | a fingerprint | For a translation: the SHA-256 of its original when it was translated; `--check` reports when it is missing or the original has changed |
| `translations`, `language_switch` | `false` or `device` | `false`: the switch offers only the versions beside the page, never the reader's browser translating it; `device`: a switch even without versions beside the page, offering the browser's translation; anything else, and no value: a switch when versions are beside the page, offering both |

Front matter that other tools use and md2html has no use for is passed over without a warning:

| Setting | |
| --- | --- |
| `tags`, `categories`, `keywords`, `layout`, `slug`, `permalink` | Passed over |
| `draft`, `status`, `created`, `updated`, `translated_by`, `translator` | Passed over |

Two limits are set only on the command line, never by a document: `--max-input` and `--max-embed`. A document cannot lift its own limits. Nor can it give the page a script: mermaid.js is named only by `--mermaid-js`, and `mermaid_js` in the front matter is refused with a warning.

### What a document may read

A document names files: the stylesheets and head files of its front matter, the original of a translation, the images `--embed` puts inside the page, the EPUB packs and the PDF prints. Each is read only when it lies, links followed, in the document's own folder or below it, or in the folder `--root` names or below it; and not at all when the document comes from standard input, which has no folder, unless `--root` names one, inside which it may then read. The folder md2html runs in is not allowed for being that: `--root .` allows it. A file outside is not read, and the document gets a warning for it that names `--root`. Only the command line gives `--root`, and `root` in the front matter or `--set` is refused, so a document cannot widen what it may read. The files the command line itself names (`--css`, `--head`, `--mermaid-js`) are read wherever they are.

## Exit codes

The numbers are those of `sysexits.h`, besides 0 and 1.

| Code | Name | When |
| --- | --- | --- |
| `0` | done | Everything asked for was done; with `--check`, nothing to fix |
| `1` | findings | `--check` found something to fix |
| `64` | usage | The command line was wrong (an unknown option, a value an option cannot take, no input), or it asked for what md2html refuses: a page, or the book or PDF its front matter asks for, over its own Markdown or into a Markdown file, one page from two inputs, several pages to standard output, `--watch` or `--serve` on standard input |
| `66` | no input | An input is not there, a folder holds no Markdown, or a folder under an input cannot be read (then no page is made); an input cannot be read, is too large or is not UTF-8 (the other pages are made); `--watch` and `--serve` end with it when every input has been gone for a while, and `--serve` at once when the folder it serves cannot be read |
| `69` | unavailable | No port was free for `--serve`; the message ends with the last port tried and why it was refused |
| `70` | software | A PDF could not be printed, or printing left something behind; the server stopped unexpectedly |
| `73` | can't create | A page, an EPUB, or a folder for them could not be written, standard output included |
| `74` | I/O error | The server could not start or its listener failed, or a message could not be written (standard output opened only for reading, or a pipe whose reader has gone, for example) in a run that otherwise succeeded. A standard output closed before md2html starts is no failure: on Linux and macOS the standard library opens `/dev/null` in its place, and the run ends as one into `/dev/null` does |

A run that meets several problems while reading inputs or writing pages goes on with the next page and ends with the code of the last one; a problem with the command line or with finding the inputs stops it before any page is made.

## Environment

| Variable | What it does |
| --- | --- |
| `SOURCE_DATE_EPOCH` | Seconds since 1970: the date a page shows and an EPUB records, instead of now, so the same Markdown renders to the same bytes. Empty is not set; anything but a whole number is a warning on each page, which is then made at the time of the run |
| `MD2HTML_GLOSSARY` | The shared glossary, a Markdown file with a `Glossary` chapter, for every page made from a file (a page read from standard input has none). None there is no shared glossary; one there that cannot be read is a warning on each page |
| `HOME`, `USERPROFILE` | The home folder: `HOME`, else `USERPROFILE` (Windows sets that one). Without `MD2HTML_GLOSSARY`, the shared glossary is `~/.config/md2html/glossary.md`, in that folder |
| `MD2HTML_GLOSSARY_URL` | Where a page's link to the full shared glossary leads, instead of the shared page's own HTML beside it; a value that is not valid Unicode is a warning, and ignored |
| `MD2HTML_CHROME` | The Chrome (or Chromium, or Edge) `--pdf` prints with, instead of the usual places; one that names no file is an error, and no other Chrome is taken |
| `ProgramFiles`, `ProgramFiles(x86)`, `LOCALAPPDATA` | On Windows, the folders where `--pdf` looks for Chrome, Chromium and Edge (`LOCALAPPDATA` for a person's own install); on macOS it looks in `/Applications` and in `Applications` in the home folder |
| `PATH` | Where `--pdf` looks for Chrome after the usual places, by its names with the system's suffix for a program (`chrome.exe`, `msedge.exe` on Windows) |
| `TMPDIR`; `TMP`, `TEMP` on Windows | The temporary folder, where `--pdf` makes Chrome's throwaway profile and removes it after the print (the standard library reads them) |

## What md2html writes

- **Files.** The page, where `-o` says or beside its Markdown as `.html`; with `--epub` and `--pdf`, the book and the PDF beside the page, with its name and `.epub` or `.pdf`. A page is written whole or not at all: through a new temporary file, renamed over the old one. `--check` writes nothing, and neither does `--serve`: it renders the pages in memory, and the book and the PDF are neither made nor offered while serving, whether `--epub` and `--pdf` or the front matter ask for them (the page says so in a warning). A file whose name is not valid Unicode (Linux allows any bytes) is read and written under its own name, but no link can name it: its page offers no download of itself and no language versions, other pages' `--nav` does not link it, `--serve` does not serve it, and the page says so in a warning.
- **Standard output.** The page, when it goes there (`-o -`, or standard input without `-o`), and nothing else; otherwise one summary line for each file written (`md2html: notes.md -> notes.html (...)`), `md2html: nothing to fix` after a clean `--check`, the help and the version.
- **Standard error.** What the author should look at, one line each: `<input>:<line>: <what>`, or `<input>: <what>` for the whole page; notes (`md2html: 3 to look at`, what is being watched or served); and errors, `md2html: <what went wrong>`.

## What is stable

What this page names is md2html's contract: the options and their spellings, the settings and their spellings, the exit codes and when each is given, the environment variables, where the files go, and which stream carries what, including the form of a warning line (`<input>:<line>: <what>`). A change to any of them is a change to this page, made in the same pull request, and named in `CHANGELOG.md`.

The anchors of a page's headings are part of it too, by their rule: the same heading text gives the same id in every version, so a link to `page.html#installation` keeps working. Letters and digits are lowercased and letters beyond ASCII kept, any other run of characters becomes one dash, the words of emphasis, code and links count as text, an explicit `{#id}` after the heading's text is taken as written, a heading with no letter or digit is `section`, and an id already on the page (a repeated heading's, or one of the page's own, such as `top`) is numbered `-2`, `-3` in order. `crates/md2html/src/tests/anchors.rs` holds the rule.

Not part of the contract: the wording of messages and of the help, the summary line's details, and the inside of what md2html writes (a page's markup, classes, styles and scripts, its other ids such as the outline's or a code block's address, the fragment's markup, the inside of an EPUB, the PDF), which may change from one version to the next.

Within a version, the same inputs, command line and environment give the same page on the same day, and the same EPUB in the same second: a page shows the date of the run unless its front matter sets `date`, and an EPUB records the moment it was written. With `SOURCE_DATE_EPOCH` set, both are the same bytes every time. A PDF is Chrome's print, and the same bytes are not promised.

What a version number promises for this contract (before 1.0, a release that only adds to it raises the patch number; one that adds a warning, so that a document `--check` passed fails it, or removes or changes any of it, the heading anchors' rule included, raises the minor number, except a fix that restores a guarantee of `SECURITY.md`, which goes out in the next patch), and which Rust md2html builds with, is [docs/policies/release_policy.md](policies/release_policy.md).
