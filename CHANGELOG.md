# Changelog

What changed in each version, newest first.

## Unreleased

- Fixed: the author's raw HTML could break the page around the article. A `</div>` the author never opened closed an element of md2html's page (and with `--html`, a `</main>` or `</article>` closed its landmarks); a `<div>` or `<details>` left open took in the notes, the glossary and the foot. The author's tags are balanced now, by default and with `--html`, in the page, the fragment and the book: an element is closed inside the element of md2html's it was opened in (the article, a list item, a quote, a callout, a footnote, a paragraph, a heading, a table cell, emphasis, a link), an end tag that closes nothing open there is left out, and an end tag closes what was opened after its element, so `<div><p></div></p>` nests. A start tag ends what HTML lets an author leave open (a paragraph before a block, a list item before the next, a table cell before the next), `<div/>` is an empty `div`, and void elements open nothing. `<details>` in one HTML block and `</details>` in a later one still hold the Markdown between them. md2html says what it changed: an end tag it left out and an element it had to close, each at its line. Pages whose HTML balances are byte for byte the same.
- Fixed: `--fragment --html` wrote an HTML block of the Markdown as written, so a `<script>` in one reached the fragment; the page with `--html` and inline tags in the fragment were cleaned, an HTML block in the fragment was not. It goes through the same filter as the page's now, and the test that no document puts running code in its page checks the fragment too.
- Fixed: a code block or an HTML block still open at the end of a text ended with one empty line too many (the text's last line ending was read as the start of another line). Five examples of CommonMark 0.31.2 were rendered wrong, and the conformance test did not see it because it ignored white space inside `<pre>`; it compares that white space now, as the specification's own test does, and all 652 examples pass. Only blocks left open at the end of a text change.
- Fixed: a translation whose `translated_from` names another file than its file name points at linked that other page in its notice, or an empty address, or itself; it links the original it names now, in that original's language, and a page that names itself as its original gets a warning instead of a notice.
- How md2html is tested is written down in `docs/TESTING.md`: each kind of test, what it guards, where it runs (commit, push, pull request, weekly, by hand) and how to add one. New: tests that run the built program as a person or a script does (where pages, books and PDFs go, each stream, every exit code, `--watch` after a save, `--pdf` with a stand-in Chrome); a seeded generator whose documents must each make a whole page, the same twice, nested, with no running code; and rules of a line and of blocks that the specification's examples leave open. Production code covered by the tests: 92 percent of lines (88 before), 86 percent of branches (83). The hostile-input tests bound a text's time by plain text's rendered beside it, not by the clock, so a busy machine cannot fail them.
- The differential test is in the repository: `just differential [base]` renders 8,300 Markdown files (the repository's, the CommonMark examples, seeded mutations of both) six ways with the build of a base revision and this tree's and compares every byte; `just generated-documents`, and a weekly job, fail when one of them makes md2html crash or run out of time.
- The inline parser, the block reader and the document assembly join the weekly mutation test, every one of their 365, 333 and 147 mutants caught; the files mutated are listed once (`scripts/ci/mutants-scope.txt`), for the weekly job and `just tier2-local` alike. Code that mutation testing showed to do nothing is gone (a rule for tildes no run could reach, conditions that restated others, the shared glossary's check for its own page); the pages are byte for byte the same.

- Faster: a page whose glossary is large renders many times faster, and its time grows with its text instead of with the text times the glossary: a text is searched only for the terms it names. A 1 MB handbook with 218 terms takes a fifth of the processor cycles it took, a 4 MB one with 872 terms a fifteenth. The page's SHA-256 fingerprint takes a third less time, the base64 of embedded images a sixth of the time it took, and the page's words for the search index are read without going through each image once more: the capabilities page, with its eleven pictures, takes 42 percent fewer cycles, a page of 55 pictures 55 percent fewer. The pages are byte for byte the same.
- Fixed: a page with many links to anchors it does not have took time growing with the square of their number (64,000 such links: 54 billion instructions); it now grows with the number (1.9 billion), and each anchor is still reported once, in order.
- Fixed: md2html reads all 652 examples of CommonMark 0.31.2 as the specification does (640 before), and the tests hold that number. A tab after a list marker counts to its next stop of four, so `-\tfoo` is a list item (it was a paragraph) and `-\t\tfoo` an item holding code. Every one of HTML's 2,125 named character references is known (`&AElig;`, `&HilbertSpace;`, `&ngE;`, which is two characters), from a table written from the WHATWG list by `just entities`; a name HTML does not know (`&MadeUpEntity;`) is text with its `&` escaped, never passed into the page. A lazy line, one that continues a paragraph in a block quote or a list item without the container's marks, is that paragraph's text whatever it looks like: not a setext underline, not a list, not a table's delimiter row. A link reference definition's label and title may span lines; `===` under nothing but definitions is text, not a heading; a blank line before a definition in a list item makes the list loose; and a backslash in a link's destination escapes only punctuation, so `\` before a space or a line break no longer carries the destination past it.
- Fixed: with `--html`, an obsolete `<dir>` or `<menu>` list was dropped and left its items outside any list; it is a `<ul>` now.
- Fixed: a non-breaking space (or any white space longer than one byte) before an attribute in an HTML code block stopped md2html with a crash; the attribute is now coloured like any other.
- Fixed: an unclosed `<` at the very end of a page's text stopped the EPUB export with a crash; it is now written as text.
- Fixed: text built to make the parser slow no longer does. Thousands of nested `[` that never close, unmatched backtick runs of many lengths and long runs of `<a ` took seconds each; each is now read once. A link label longer than 999 characters is no label, as the CommonMark specification says, for link and footnote definitions alike.
- Fixed: a file read by md2html is read through its size limit whatever its size claims, so a named pipe, or a `.md` link to an endless device in a folder md2html walks, is refused instead of read without end. The pages are written through a temporary file created new, so a link planted under its name is not followed.
- Fixed: `--serve` answers at most 64 connections at once, gives a request ten seconds to arrive, and ends a reload stream after ten minutes (the page reconnects by itself); `--watch` and `--serve` stop when every input is gone. The server's guarantees (local names only, nothing outside the folder, GET and HEAD only) have tests now.
- Fixed: an image address whose percent escapes decode to bytes that are not UTF-8 names no file; before, it could name a different file whose name held U+FFFD.
- Fixed: `</scripts>` no longer ends a `<script>` element that the sanitizer drops, and `-MOZ-BINDING` is refused in a head file's style like `-moz-binding`.
- Fixed: the text-size buttons' titles ended in a stray comma in eight languages ("Smaller text,").
- Fixed: `--nav` over nested folders wrote some links to pages not yet written as absolute file paths; they are relative now.
- md2html exits with its documented codes from `main` (a message that cannot be written ends a run with 74 instead of a crash); a bad `--port` names the parse error.
- The code holds the whole lint table with no exception but one: no `#[allow]` or `#[expect]` anywhere, tests and tooling included, except floating-point arithmetic in the diagram geometry (`mermaid/geom.rs`), and a gate refuses any other. Every module was rebuilt to remove the causes of the 2,051 findings the table had found: a reader that keeps its own place instead of positions into text, the page model as one type per kind, typed records instead of parallel vectors and tuples, named steps instead of long functions. Every file is under 800 lines. 49,788 renders of 8,298 documents came out byte for byte the same, apart from the comma above.
- The sanitizer and the text helpers have tests that catch every mutation testing makes (159 escaped before), and the weekly mutation run is a required check over all their files.
- The build tooling runs on the standard library and six crates: 28 crates in all instead of 90, every one fully audited, none exempted, four licences.
- Every gate is required: no gate passes because its tool is missing, no switch turns one off, and the hooks run every check they name (the signature check never ran locally before).
- A push checks md2html on the Rust it declares as its minimum (1.85), with the flags CI uses, so a finding there stops the push instead of the pull request.
- The minimum Rust is written once, in md2html's own manifest: CI's `msrv` job reads it there instead of keeping its own copy, and the root `Cargo.toml` says why only md2html declares one (the tooling is built with the pinned toolchain). The manifest gate holds that rule, and now reads every dependency table (`[build-dependencies]`, `[dev-dependencies]`, per-target tables), not `[dependencies]` alone, so a version written into a crate's manifest is refused wherever it is.
- Fixed: md2html's EPUBs failed epubcheck, the W3C's validator; every one of the repository's 19 books did. They pass now, with no error and no warning. The table of contents wrote an empty list for a chapter without sections (an `ol` must hold an `li`); the style sheet set `direction` and `unicode-bidi`, which EPUB forbids, so code, keys, maths and diagrams now carry `dir="ltr"` themselves; a link to a file the book does not hold (another page, a PDF, a `file:` address) keeps its text without the link, and md2html names each target once; a figure no longer links to its own image, which EPUB allows only for documents; an image on the web, or one that cannot be read, gives way to its text alternative instead of pointing out of the book; and with `--html`, a video's, audio's or `<source>`'s file that is not in the book is left out and named. The web pages are byte for byte the same.
- Fixed: with `--html`, HTML 4's presentational markup that README files still use made an EPUB invalid. In the book it now says the same thing the way EPUB's XHTML does: `align` on a block is a class of the book's style sheet (left, centre, right, justified), on an image a float or a vertical alignment, on a table or a rule its margins; `<center>` is a `div` (a `span` inside text) with that class and `<strike>` an `s`; `width`, `height`, `bgcolor`, `cellspacing`, `nowrap`, `hspace`, `vspace`, `clear` and a list's `type` are styles, and a table's `border` is `1`. What a book has no equivalent for (`cellpadding`, `summary`, `frame`, `rules`, `noshade`, a rule's `size`, a cell's `axis`, `scope` or `char`, `longdesc`, `compact`, a preformatted block's `width`) is left out and named once per kind. Inline SVG and MathML keep their attributes. The web page keeps the markup as it is.
- CI checks the books: the `epub` job renders the README, the showcase, the capabilities page and its translations as EPUBs and runs epubcheck 5.4.0 (the W3C's release, checksum verified) on each, failing on any error or warning, and it is a required check. `just epubcheck` does the same locally; `just install-dev-tools` installs epubcheck.
- md2html's modules stand in layers, from reading text up to the command line, written down in `docs/ARCHITECTURE.md`: a module uses its own layer or a lower one, and a test fails on a use of a higher layer or a circle of uses. Three such circles are gone; the handful of helpers that closed them live in the layer they belong to (character references and JSON escaping in the text helpers, the raw-HTML tag reader and safe list in the sanitizer, the run's exit codes and messages in a module the command line and the server share). The pages are byte for byte the same.
- Every item of md2html and of its build tooling is visible only as far as the code that uses it, as the compiler confirms item by item: 104 items and fields that only their own module uses are private now, 43 reach only their parent module, and the two tests that had made an item visible across the crate sit beside the code they check. The pages are byte for byte the same.
- What the command line promises is written on one page, `docs/command-line.md`: the inputs and where the pages go, every option with its aliases (`--output`, `--self-contained` and `--unsafe-html` among them, which the help does not show), every front matter setting with its other spellings and the keys md2html passes over, the exit codes and when each is given, the environment variables, which stream carries what, and what is stable. A test holds the page and the code to each other both ways, so an option, a setting, an exit code or an environment variable cannot be added to one without the other.
- Fixed: `md2html --help` said `--lang` takes "en (default) or de" and showed no option's other spellings. It now names the seventeen languages a page's own words come in (and that the file's name, `notes.de.md`, gives the language when `--lang` does not), shows `--output`, `--self-contained` and `--unsafe-html` on the line of `--out`, `--embed` and `--html`, says that a folder means every Markdown file under it, and that `--quiet` keeps the warnings `--check` reports. A test holds the help to `docs/command-line.md`: every option of the page is in the help with all its spellings on one line, and the help shows none the page does not list.
- A value with a rule keeps its fields to itself: a date (made only from the clock or an ISO date), the glossary's link counter (one link per term and chapter, every mention numbered) and a sequence diagram's column (activations that open and close like brackets) are changed only through their own methods now, as is the release pipeline's resumable state. Each module's root file is the one place the rest of md2html uses it through, and the layer test now holds that; the developer tools' library is four private modules behind one root. The type policy says which structs show their fields and why, where `#[must_use]` goes in a binary, and why md2html has no use for `#[non_exhaustive]` or sealed traits. The pages are byte for byte the same.
- Text and HTML are different types inside md2html now: the title, brand, kicker, description, author and the other words a page says about itself are escaped once on their way into the page, and markup cannot be escaped a second time or written without escaping, because the compiler tells the two apart. A test writes every place text enters the page around the article with `<`, `"` and `&` and finds each escaped exactly once. The settings with a rule (`mermaid`, `translations`, `source_sha256`, `live`) are read once into a value of their own, from the front matter and the command line alike; a heading's level is one to six by construction; and where a `true` or `false` picked a way of working (how a list is spaced, whether a URL may be an image's pixels, how the parser reads `---`), the code names the way. The pages are byte for byte the same.
- Failures md2html used to hide are reported. A folder under an input that cannot be read ends the run with 66 before any page is made (it was taken for an empty one, and an unreadable input folder was said to hold no Markdown). A shared glossary that is there but cannot be read, `SOURCE_DATE_EPOCH` set to anything but a whole number of seconds (the output was silently not reproducible), and `MD2HTML_GLOSSARY_URL` that is not valid Unicode are warnings on the page, which `--check` counts. An image the EPUB cannot read is named as such instead of "not found", and `--serve` says why the last port it tried was refused. A path without a file name (`-o ..`) is refused instead of written to `..md2html-tmp`.
- Fixed: a file whose name is not valid Unicode (Linux allows any bytes) was handled through text with U+FFFD in place of its bad bytes. Every decision about a name (an option or an input, hidden or not, its place in a folder, a stylesheet or not, a served file's type) is made on the name itself now, and the PDF is printed from the path's own bytes. No link can name such a file, so its page says so in a warning and offers no download of itself and no language versions (a stem read as empty had taken `.de.md` beside it for the German version), `--nav` does not link it, and `--serve` does not serve it (two such pages had shared one address). Unicode names are not affected; the pages are byte for byte the same.
- The anti-pattern gate refuses `to_string_lossy` in production code, beside `from_utf8_lossy` and `from_utf16_lossy`. The panic policy now says which errors each kind of crate returns (messages and exit codes in md2html, `anyhow` with context in the tooling, typed errors in the two libraries) and when a default is an answer rather than a hidden failure. The developer tools' library returns typed errors that keep their cause, and `release-local` no longer stops with a panic on an argument that is not Unicode.
- Faster, with less memory, and measured: a page with a glossary no longer works out every term's spellings again for each piece of its text, so a 1 MB handbook with 218 terms takes a quarter of the work (5.1 billion instructions instead of 19.1) and 316 thousand allocations instead of 21 million. The page is written into one string instead of being copied together from three, the search index from the page instead of from a copy of it, a table's cells into the table, and escaped text and an embedded image's data straight to where they go: the capabilities page, with its images, allocates 18 MB instead of 34. `just measure` times the stages of a render (the blocks, the bare HTML, the whole page) on the repository's documents. The pages are byte for byte the same.
- Every trait, generic parameter and standard trait implementation of md2html and its tooling was checked against the type policy, and what did no work is gone: the build pipeline's colours are functions instead of a trait implemented once, the flowchart layout sorts its numbers by a total order (a NaN, which no diagram produces, would have been "equal" to every number and could have made the sort panic), a conversion that copies is named `to_`, and a default is a real value. The policy now says when a trait, a generic parameter, an `Ord` and a `Default` earn their place. The pages are byte for byte the same: 49,788 renders of 8,298 documents.
- The features and dependencies of the workspace were checked crate by crate. md2html still has none. The tooling's `regex` is built with the two features its patterns use instead of all its defaults, so 27 of the 28 locked crates are built and the workflow checker is a third smaller; the one feature of the workspace, `build` in `md2html-version`, has a test of what it writes and its contract in the crate's documentation; no crate is in two versions and every dependency is used. The pages are byte for byte the same.
- Fixed: `--serve` reads a request by the rules of HTTP/1.1 and refuses what it cannot read with the status that says why. A malformed request line was answered 403, 404, 405 or even 200 (`GET /a.html HTTP/1.1 junk` served the page); it is 400 now, as are a request without a `Host` or with two, a space before a header's colon and a header that is not one. A request line over 8 KiB is 414 instead of being cut and its cut part served, a header over 8 KiB or more than 64 headers 431 instead of being read on or ignored, a request that stops coming 408 instead of no answer, an HTTP other than 1 505. A header that is not UTF-8 no longer drops the connection: only the `Host` has to be text. A refusal reaches the browser even when it sent more than was read (a POST with a body was answered with a reset). Requests from a browser are answered as before.
- Fixed on Windows: a document could make the EPUB pack a file from outside its folder (`/secret.png` and `C:secret.png` are not absolute there, so they were read from the drive's root or its current folder), and `--serve` served a hidden file asked for with a backslash (`/notes%5C.git%5Cconfig`). Both are judged now on the path's parts as Windows reads them and on where the path leads: an EPUB image, like an embedded one, only from the page's folders, links followed; and the server serves no hidden file, whatever leads to it (on every system, a visible link to a hidden file is not served any more, nor packed an image that links out of the folder). A path ending in a separator names a folder on every system, by md2html's rule rather than the file system's: Windows opens `pic.png/` as `pic.png`, so the server, the EPUB and `--embed` refuse it as a file everywhere; and the server serves a file only under its own names, not with an empty or a `.` part (`//pic.png`, `a/./pic.png`).
- Fixed on Windows: `--pdf` gave Chrome a `file:` address it could not open (`file:////%3F/C:/...`, from the path's verbatim form), never found Chrome or Edge on `PATH` (the names lacked `.exe`), and missed Chrome installed for one person (`LOCALAPPDATA`) or on another drive than `C:`; the shared glossary was never found by default, as Windows sets `USERPROFILE`, not `HOME`, and its link was a Windows path after `file://`. On macOS `--pdf` also looks in `~/Applications`. `MD2HTML_CHROME` set to what is not a file says so, instead of saying no Chrome was found. Between pages on two Windows drives, `--nav` links by `file:` address instead of a link a browser cannot follow. On Linux and macOS a backslash in a file name is part of the name again: it was read as a separator in links, served addresses and `-o`.
- Fixed: `--watch` and `--serve` missed a save made within a second or two of the last one on a file system that keeps time coarsely (FAT, exFAT, HFS+, many network shares); a change of size counts now as well as a change of time.
- Where Linux, macOS and Windows differ is written down in `docs/policies/platform_policy.md`, with the test that holds each system's rule on that system; md2html's own rules for it are in one module, `system`, and no production code in the workspace has a `cfg` on a system. A document with Windows' line endings, a name differing only in case, and file names that are not Unicode on Windows now have tests.
- `--serve` stops whole: every thread has an owner that waits for it, and when the inputs are gone the server ends every reload signal, closes every open connection and waits for every thread before md2html exits. A thread that fails is told, not lost. The concurrency policy is one page: each thread, its owner, how it ends, what is shared and why, every limit, every answer; tests over real connections hold the limits, the refusals and the stop.
- The release pipeline gives every program it runs a time limit: a minute for a question to `git`, `rustc`, `df` or `sccache`, five for `gh` and `cargo update`, which go over the network and could hang `just ship`. A `cargo update` that fails after the version bump fails the bump instead of being passed over.
- Every generated file is checked and made from the repository alone. The table of HTML's named character references was written from the Python on the machine and its check ran nowhere; it is now written from the WHATWG's list, vendored unchanged in `scripts/entities/entities.json` (the list is static by the HTML Standard's word; its checksum is recorded and any other bytes are refused), with `\n` line ends on every system, and the gate `entities-drift` checks it on every push and pull request. The tooling's `--version` stamps only what its build scripts declare to cargo: the commit of the crate's own repository (a git hook's `GIT_DIR` no longer leaks in), followed after a commit even when the branch is packed, and no `-dirty` suffix, which no build could keep true. Every macro says why it is one and shows an example, and the generated hooks point at a document that exists. The pages are byte for byte the same.

## 0.7.7 — 2026-11-23

- The capabilities report is now `docs/capabilities.md`, a tour of everything md2html does for anyone who comes to the repository, beginning with where it runs; written afresh, with every screenshot retaken, and in all seventeen languages. The README opens the same way: where it runs, what it does, then install.
- The repository is a rust-forge workspace now (template 1.1.3, adopted with its `adopt.sh`): md2html lives in `crates/md2html`, beside the forge's build tooling (the `just` pipeline, the generated git hooks and gates, the CI and release workflows, nextest profiles, the deny, vet, typos, taplo, REUSE and file-size policies). md2html itself still depends on nothing; the tooling's crates are vetted with the forge's cargo-vet store. A plain `cargo build`, `cargo test` and `cargo install ... md2html` mean md2html.
- md2html's code meets the forge's lint posture: clippy's pedantic, nursery and cargo groups and documented private items, with a written reason wherever a rule does not fit. The pages it renders are byte for byte the same.
- The release, packaging and preview-build workflows are in place and dormant: each waits for its repository variable (`LANE_RELEASE`, `LANE_BREW`, `LANE_WINGET`, `LANE_CRATES`, `LANE_SLSA`), a manual dispatch, or a label on a pull request.
- `just use-local` builds md2html from the checkout and installs it to `~/bin`; `just release-local` builds the release archives for six platforms from one machine (macOS, Linux as a static binary, and Windows, each on ARM64 and x86-64), checks every binary, and writes their SHA-256 checksums. Both are a small workspace tool (`scripts/dev-tools`) with no dependencies.
- The audit store names every audit's auditor the same way and no longer imports audits from another repository.
- The whole lint table of the forge is on at its own levels (some two hundred individual lints at deny, where the adoption had left them at allow). New code meets it; the 2,051 findings md2html's code had that day are recorded at the top of each file and can only go down (`GOTO.md` lists them).
- The scaffolding names md2html where it still named the template's sample crates and a placeholder account: the weekly deep suite, the failure notifier, `just version`, the packaging and analysis recipes. The forge's own guides, a contributing guide, `GOTO.md` (where the project stands) and the release configuration are in the repository.
- Pull requests are gated in full: with no merge queue to run them, the heavy lanes (clippy, tests, rustdoc, deny and vet, Windows) run on the pull request, and `main` requires them together with md2html's own tests on three systems, the oldest supported Rust, the supply chain and the browser suite.
- Fixed: with `--html` and `--epub`, an image address in which a `%` was followed by a hexadecimal digit and a non-ASCII character (`<img src="%aé.png">`) stopped md2html with a panic, against its promise that no input can; and a `data:` image written with percent escapes instead of base64 lost every byte above 127 on its way into the EPUB. Both came from three copies of one percent decoder, which are now one, exact to the byte. The forge's anti-pattern gate found them; it now runs in CI too. Pages and EPUBs of ordinary documents are byte for byte the same.
- Dependencies brought up to date: the toolchain is `nightly-2026-11-23`, the newest on which every lint lane passes for macOS, Windows and Linux; the tooling's crates are at their latest versions, and each of the thirteen that was younger than a week was read as a diff and its audit recorded. Dependabot waits a week before proposing any release, for crates and actions as for the browser tools.
- The pipeline tool no longer uses `chrono`, which the advisory database lists as unmaintained: it reads the clock itself, in UTC. Sixteen crates left the lockfile with it, the audit store's exemptions fell from 18 to 9, and `deny.toml` ignores no advisory.
- md2html builds for ARM64 Linux: it has its own zig (0.17.1, installed checksum-verified beside whatever zig is on the PATH), and `cargo update` may take the tooling's crates past md2html's own minimum Rust, which has no crates to hold back.
- Tested on Windows too: CI builds, lints and tests on macOS, Linux and Windows, and text stays LF on every checkout.

## 0.7.6 — 2026-11-22

- Diagrams and code blocks have addresses of their own (their section and kind: `4-code-and-maths-diagram`), and figures, diagrams and code blocks show the share symbol in their corner under the pointer (beside a code block's copy button, not on it).
- At a larger text size the share symbols and the passage offer were drawn off the screen: the text size zooms the page's body, which scaled their coordinates. They now hang outside the zoomed body, and follow their element when late images move it.

## 0.7.5 — 2026-11-22

- A heading's link is a share symbol (a box with an arrow out of it) to the right of the heading, instead of a # in the margin at its left; it shows when the pointer is on the heading or it has keyboard focus, and always, faintly, on a touch screen.
- The offer to link a selected passage carries the same symbol, in the accent colour.
- Table rows have addresses of their own, from their section and first cell (`#7-settings-lang`), and the share symbol at the end of the row under the pointer; the link opens the page on that row, highlighted.

## 0.7.4 — 2026-11-22

- No change in what md2html does. The report's site build now compiles md2html before rendering and stops unless the binary, `Cargo.toml` and the report's own `version` agree: 0.7.3 went out with a 0.7.2 binary, so the credit and the footer named different versions.

## 0.7.3 — 2026-11-22

- With the outline folded away, the page could slide back by a screen or more while the reader scrolled: the outline's scroll-spy kept its marked entry in view with `scrollIntoView`, which also scrolls the page, toward the hidden outline. The outline and the search results now scroll only their own box. A language switch then carried that wrong spot, which looked like a jump to the top.
- The language switch finds the spot from the block at the top of the screen, whichever way the reader got there (an outline jump left the heading lower than the old rule looked), and carries it in the tab's session storage instead of the address.
- The "Skip to content" link shows only for keyboard focus, not after a click or a page load.

## 0.7.2 — 2026-11-22

- After a reload or a language switch, the spot is held while the page settles: images above it that load late would push it away, and Safari does not anchor the scroll itself. Until the reader scrolls, or for a few seconds.
- A live page's check reads only until it has seen the page's fingerprint, then stops the download, also from servers that ignore the Range request.

## 0.7.1 — 2026-11-22

- Switching language keeps the exact spot: the same paragraph (or table, list, figure) at the same height on the screen, not only the same chapter. The link carries the heading's place, the block's place under it and its height (`#~H.B.O`); a translation keeps both. The spot is set again once the page has loaded, so images above it cannot push it away.

## 0.7.0 — 2026-11-22

Hardening: md2html cannot be used to make a page that does harm, and no input can stall or crash it. [SECURITY.md](SECURITY.md) states the guarantees.

### Security

- `--html` no longer passes raw HTML as is. Rich HTML (images, links, video, audio, SVG, MathML, tables, layout) passes an allowlist; scripts, event handlers, `javascript:` URLs, frames, plug-ins, forms, `<meta>`, `<base>` and styles that fetch or run anything are removed, the dangerous elements with their contents, and ids the page's own script uses are not taken. **Pages that relied on `--html` to carry scripts lose them**, by design.
- The author's `head` file keeps only `meta` (without `http-equiv`), `link` (stylesheets, icons and the like) and `style`.
- `brand_link` refuses a `javascript:` or `data:` address.
- Files the front matter names (`css`, `head`, `mermaid_js`, `translated_from`) must lie inside the document's folder and be of their kind; the command line may still name any file.
- An embedded image must lie in the document's folder or the folder md2html runs in, and its bytes must be an image (PNG, JPEG, GIF, WebP, AVIF, ICO, BMP or SVG), whatever its name says: a document cannot pull a secret into its page.
- `--pdf` prints with every network connection refused.
- GitHub private vulnerability reporting is on.

### Robustness

- A run of 100,000 asterisks overflowed the stack and aborted; emphasis, links and images now nest at most 64 deep.
- The emphasis pass, the bracket matcher and the reading of link definitions were quadratic: 25,000 `*a*` in one paragraph took 26 s, 50,000 took 112 s, 80,000 link definitions 1.5 s. All are linear now (100,000 `*a*`: 0.2 s; 80,000 definitions: 0.15 s), as CommonMark's reference implementations do it.
- A Markdown file over 64 MB and an image over 32 MB to embed are refused with a clear message; `--max-input MB` and `--max-embed MB` raise them, on the command line only.
- The test suite renders pathological inputs under a time limit on every change.

### The bar

- Saving has its own button, a download arrow at the end of the bar, with its own menu (the page, print or PDF, the PDF and the EPUB); **Aa** holds the text size and the theme only.

### Live pages

- `--serve [--port n]` serves the pages on this computer and reloads every open page the moment its Markdown is saved: loopback only, this machine's names only (no DNS rebinding), the document's folder only, GET and HEAD only.
- `--live` (`live: true`, or `live: 30` for a time in seconds; `--ttl`) gives a page its own time to live: on a server it then fetches the first kilobyte of itself and reloads if its fingerprint changed; opened from a file it simply reloads. Every page now carries that fingerprint (`<meta name="md2html-page">`). With `--csp`, a live page may connect to its own origin only.
- A reload keeps the reader's place, best effort: the block at the top of the screen by its text in its section, then anywhere, then the closest text or position in the section; else the section before; else the top. It waits while the reader is selecting or has a menu, search or a figure open, and says "Updated" afterwards (in the page's language).

### Sharing

- The **#** beside a heading copies the section's link and says so ("Link copied", read aloud too); on a phone it opens the system's share sheet.
- Selecting words offers a link to exactly that passage, a text fragment (`#:~:text=`) that opens the page there, highlighted.

## 0.6.0 — 2026-11-22

Languages.

- The page's own words in seventeen languages: English, German, Hebrew, Indonesian, Dutch, Javanese, Yiddish, Low German, French, Italian, Spanish, Japanese (in kanji and kana, `ja`; in kana alone, `ja-Hira`; in rōmaji, `ja-Latn`), Mandarin Chinese (simplified, `zh-Hans`; traditional, `zh-Hant`) and Cantonese (`yue`) (search, glossary, callouts, footnotes, figure, theme and text size, dates). "How to read this", "Glossary", "Contents" and the callout words are recognised in each.
- Translations: `report.de.md` beside `report.md` is its German version. md2html links the versions with a language switch in the bar (a native popover; the reader keeps their place) and with `hreflang`; each translation tells the reader the original is authoritative.
- A translation records `translated_from` and `source_sha256`; `--check` says when the original has changed since, and gives the new fingerprint.
- Hebrew and Yiddish (and Arabic) are laid out right to left: the outline on the right, callouts and quotes edged on the right; code, maths and diagrams stay left to right.
- Where the reader's browser has an on-device translator (Chrome's built-in Translator API), the switch also offers every other language, translated on the device itself, labelled as a machine translation, with a way back to the original.
- Chinese, Japanese and Thai glossary terms are found inside running text, which puts no spaces between words.
- The capabilities report exists in all seventeen languages.

Reading, keeping, and credit.

- The bar is calmer: search, the language globe, and **Aa**, a panel with the text size, the theme (System, Light and Dark side by side) and *Save or print*.
- *Save or print* offers the page itself (when it reads offline), printing or saving as PDF, and the PDF and EPUB beside the page when they were made.
- `--pdf` (or `pdf: true`) prints the page with the Chrome on the computer: its headings become the PDF's bookmarks, every page carries the title and its number. Headless, a throwaway profile, a time limit; a clear message when there is no Chrome.
- `--epub` (or `epub: true`) writes an EPUB 3 of the page: XHTML, the outline as contents, the images packed inside, diagrams and maths kept, footnotes as e-reader notes, right to left where the language is. Written by md2html, no dependency.
- Printed pages carry the title at the head and the page number at the foot.
- Every page ends with *Made with md2html* and its version, linked to md2html's home.

### Fixed

- Tables, figures and diagrams broke out of the reading column to the width of the screen: a sized figure or a table of long sentences sat against the left edge, and at a larger text size a table ran past the right one. They stay in the column now, and scroll there when they are wider.
- On a phone the bar's buttons now always fit.
- A glossary chapter is recognised by its whole heading: "## 5. The glossary and the footnotes" is a chapter again, not a second glossary.
- `-o folder/` with a single input writes into the folder.
- A diagram's description for screen readers no longer joins its parts with English words.

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
