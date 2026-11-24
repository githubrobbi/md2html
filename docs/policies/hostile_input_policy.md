<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Hostile input: the bugs the compiler does not catch

md2html reads Markdown that someone else may have written and makes a page that other people open. Rust rules out whole classes of bugs (memory errors, data races), and md2html forbids `unsafe` code. What it cannot rule out is a program that does exactly what it was written to do, with input its author did not imagine. This document lists those classes for md2html, how the code holds each, and which test or gate checks it.

[SECURITY.md](../../SECURITY.md) states what md2html promises to the people who use it. This is the same ground from the side of the person changing the code. A change near `sanitize.rs`, `text.rs`, the embedding of files or the server comes with a test that shows the guarantee still holds ([AGENTS.md](../../AGENTS.md), section 11).

## 1. Text that becomes code in the page

The compiler sees strings; it does not know that `<`, a quote or `javascript:` mean something to a browser. Every byte of a document that reaches the page has to arrive as text, never as markup or script.

How the code holds it:

- Text goes into the page through `text::esc`, which escapes `&`, `<`, `>` and `"`, in element content and attribute values alike.
- A link or image address goes through `text::safe_url`, which refuses script schemes (`javascript:`, `vbscript:` and their kin) in any case and with white space or control characters inside, and `data:` addresses except an image's. A scheme with some of its letters written as character references is refused as well.
- Raw HTML in a document is shown as text, except a short list of harmless tags. With `--html`, rich HTML passes through `sanitize.rs`, an allowlist of elements, attributes and style properties: what is not on the list is removed. There is no denylist to fall behind.
- The author's tags are balanced (`sanitize::Balance`): every element md2html writes around the author's text is a scope, what the author opens in it is closed before md2html closes it, and an end tag that matches nothing open there is left out. Raw HTML cannot close an element of the page (its landmarks, the outline, the foot, the page's script's targets) or leave one open to take in what follows, and the EPUB's XHTML stays well-formed.
- The author's `head` file keeps only `meta` (without `http-equiv`), `link` and `style` (`sanitize::head`).
- Data the page embeds for its own script, the glossary among it, cannot close the element that carries it.
- `--csp` adds a content security policy that lets the page's own script run and nothing else.

Checked by: the tests `script_links_are_never_written`, `raw_html_cannot_run`, `the_page_cannot_be_broken_out_of`, `no_document_puts_running_code_in_its_page`, `rich_html_keeps_what_is_harmless` and `the_csp_lets_exactly_the_page_run` in `src/tests/hostile.rs`; `the_page_around_the_article_keeps_its_shape` in `src/tests/balanced.rs` and the generated documents (`src/tests/generated.rs`), every element of the page nested; the tests of `sanitize.rs` and `text.rs`; Miri over those tests and mutation testing of the modules in `scripts/ci/mutants-scope.txt` (`sanitize` and `text` among them) in the weekly run ([lint-posture.md](lint-posture.md), "The weekly checks"); `tests/browser/audit.js` and `interact.js` in CI on the rendered pages.

A new place where document text reaches the page uses `esc` or the sanitizer, and its test feeds it an attack string.

## 2. Reading files a document should not

A document can name files: a stylesheet, a head file, a translation's original, images to embed. Its author is not the person running md2html, so a document must not be able to pull `~/.ssh/id_rsa` into a page.

How the code holds it:

- A file named in the front matter must be relative, have the extension of its kind (`.css`, `.html`, `.js`, `.md`), and lie inside the document's folder (or the folder md2html runs in). Otherwise it is not read, and the reader gets a warning.
- The check is made on the path the file system resolves, not on the text: `text::inside` canonicalises both the file and the folder, so `..`, symbolic links and odd spellings are judged by where they really lead. Two strings that differ can name the same file, and a check on the string alone can be walked around.
- An image is embedded only from those folders, only up to the embedding limit, and only when its first bytes are those of an image. An image packed into the EPUB is held to the same folders, on its resolved path, and read the same way, through the same limit and by its bytes (`render::read_image`, `export/images.rs`); its address must be names only, read with the system's separators, so on Windows neither a drive's root (`\x.png`) nor a drive's current folder (`C:x.png`) is read.
- The server serves only files inside the document's folder, judged the same way, on the resolved path: no way out by `..`, by its percent-encodings or by a link, and no hidden file, judged both on what is asked for, read with the system's separators (on Windows `\` and `%5C` separate as `/` does), and on what it resolves to (a visible link to a hidden file, Windows' short name `GIT~1` for `.git`); and only under the file's own names, by md2html's rule on every system: not as a folder (`picture.png/`, which Windows opens as the file), not with an empty part or a `.` part (`serve/http.rs`, `system::file_below`).
- The command line and the environment belong to the person running md2html and may name any file.

Checked by: `a_document_cannot_read_files_outside_its_folder` in `src/tests/files.rs`, which tries a stylesheet outside the folder, `/etc/hosts` as a head file, and a secret named like an image; `an_image_outside_the_folder_is_not_packed`, `on_unix_a_link_out_of_the_folder_is_not_followed` and `on_windows_a_drive_or_its_root_is_outside_the_folder` in `src/export/images.rs`; `nothing_outside_the_folder_or_hidden_is_served`, `a_link_out_of_the_folder_is_not_followed` (Unix) and `on_windows_a_backslash_does_not_hide_a_hidden_file` (Windows) in `src/serve/tests.rs`.

## 3. Input that exhausts time, memory or the stack

A document can be built to make a parser work very hard, or to make md2html wait: a hundred thousand `[` that never close, emphasis delimiters that a naive parser matches in quadratic time, quotes nested ten thousand deep, a formula of braces inside braces.

How the code holds it:

- Size: every file md2html reads, and standard input, is read through a limit (`input.rs`), never by asking the file system how large it is and trusting the answer: a pipe, a device behind a link or a file that grows while it is read reports a size it does not keep to. An input over 64 MB is refused (`--max-input` raises it). The other files a page is made from (a stylesheet, a head, mermaid.js, the original of a translation, the shared glossary) are refused over 64 MB (`input::MAX_BESIDE`). An image over 32 MB is neither embedded nor packed into a book (`--max-embed`).
- Waiting: a file a document names is read only when it is a file (`input::read_named`): a named pipe, which md2html would wait on for ever when it opened it, a device or a folder is refused with a warning. What the person names on the command line may be a pipe (`md2html <(...)`). Only the command line can raise a limit: `max_embed` in the front matter is refused.
- Time: the CommonMark parsers are linear. Emphasis and links are resolved with the specification's delimiter stack, not by searching back from every closer. So is the work after them: the check for links that lead nowhere looks each link up in sets (the page's ids, the anchors already reported), and a text is searched only for the glossary terms it names (a term is indexed by its first word, or its first character when it may stand inside a word), so a glossary of thousands of terms costs a text no more than its words.
- Depth: every nesting has a bound, so recursion cannot overflow the stack. Inline nesting of emphasis, links and images stops at 64 levels (`inline::MAX_NEST`), block containers at 64 (`block::MOST_DEPTH`), TeX at 60 levels and 20,000 characters (beyond that the formula is shown as TeX), a flowchart at 400 nodes (beyond that it is shown as code).
- The server bounds what a browser can make it do: at most 64 connections at once (the rest are told it is busy), 10 seconds for a request to arrive whole, 8 KiB for the request line and each header, 64 headers, 10 seconds for each write, and a reload signal that ends after 10 minutes. A request over a limit is refused, never cut and read on: 414 for a request line too long, 431 for a header too long or too many, 408 for one too slow; what is not a request at all is 400 ([concurrency_policy.md](concurrency_policy.md)).

Checked by: `pathological_inputs_render_quickly_and_whole` and `pathological_brackets_finish_quickly` (each case within a time limit) in `src/tests/hostile.rs`; `a_text_is_searched_only_for_the_terms_it_names` in `src/glossary.rs` (by size: a text naming five of 20,004 terms is searched for those five) and `many_links_to_nowhere_are_each_reported_once` in `src/document/checks.rs`; `limits_are_lifted_only_on_the_command_line` and `an_input_is_read_through_its_limit_whatever_it_claims` in `src/cli/tests.rs`; `only_a_file_is_read_when_a_document_names_it` in `input.rs`, `a_book_image_is_read_through_its_limit_and_by_its_bytes` and `a_named_pipe_is_not_read_as_an_image` (Unix) in `src/export/images.rs` and `text_is_read_up_to_its_limit_and_no_further` in `input.rs`; `hostile_flowcharts_stay_bounded` in `src/mermaid/tests.rs`; `a_malformed_request_is_answered_400` and `a_request_over_its_limits_is_refused_with_its_status` in `src/serve/request/tests.rs`; `refusals_reach_the_browser_whole`, `a_slow_request_is_answered_408_and_a_silent_one_closed`, `connections_beyond_the_limit_are_turned_away` and `the_server_stops_with_connections_open_and_leaves_nothing_behind` in `src/serve/tests/tcp.rs`, over real connections.

A new parser, or a new loop or recursion over input, gets a bound and a test that feeds it a pathological input under a time limit, or better, a test that holds the bound by size: how much work one part of the input can cause, whatever the size of the rest.

Beyond the tests, by hand and weekly: `just generated-documents` renders a corpus of about 8,300 documents, generated and mutated, six ways each, and fails when one panics, ends by a signal or runs past three minutes; `just differential` compares the same corpus byte for byte with the build of `origin/main` ([TESTING.md](../TESTING.md)).

## 4. Writing where it should not

How the code holds it:

- md2html refuses to write over its input, a link to its input included, and refuses an output whose name is a Markdown file (`unsafe_output` in `cli/files.rs`).
- A page is written to a temporary file beside its destination and then renamed over it, so a reader never sees half a page (`write_atomic` in `cli/files.rs`). The temporary file is created new (`create_new`): a link planted under its name is not followed, so nothing is written through it. A PDF is printed to a hidden file beside its destination, checked to be a PDF, and renamed over it.
- `--pdf` runs Chrome headless with a fresh profile in the temporary folder, a time limit, and every network connection refused.

Checked by: `a_run_refuses_to_write_over_its_input_or_a_markdown_file`, `an_output_that_links_to_the_input_is_the_input`, `a_file_is_written_whole_and_a_planted_link_is_not_followed` and `a_link_planted_at_the_temporary_name_is_not_followed` in `src/cli/tests.rs`.

## 5. The same input, different output

Pages are compared, not inspected by eye: a change that is not meant to alter output must leave `README.html`, `docs/showcase.html` and `docs/capabilities.html` byte for byte the same ([AGENTS.md](../../AGENTS.md), section 11). That only works if the same input always gives the same bytes.

How the code holds it:

- No iteration in hash order: maps and sets are `BTreeMap` and `BTreeSet`, and `iter_over_hash_type` is denied.
- The clock enters the page in one place, `text::now_secs`, and `SOURCE_DATE_EPOCH` replaces it. The date is worked out from those seconds by `text::civil`, in UTC, with no time zone and no locale. The EPUB's entries carry a fixed date (1980-01-01).

Checked by: the lint; `the_same_documents_render_the_same_bytes_anywhere` in `src/tests/determinism.rs`, which renders the same documents twice, from different folders, and compares every byte; the comparison of the three pages, made before a pull request.

## 6. Bytes read as text

A decision made on text that was decoded with replacement characters is a decision about different bytes than the ones on disk or on the wire.

How the code holds it:

- Input is read as UTF-8 and refused when it is not (`input::read_limited`, `Unread::NotText`).
- A place in a text is kept by `crate::scan`, which only ever stops on a character boundary (`string_slice` is denied).
- Where bytes decide (is this file an image, which file does this URL name), the decision is made on the bytes, or on what the file system resolves.
- A file name is not text either: on Linux it may hold any bytes. A decision about a name (is it hidden, an option, a stylesheet, which page is this) is made on the `OsStr` or `Path` itself: comparisons, `as_encoded_bytes` for ASCII tests, `Path::extension` with `eq_ignore_ascii_case`. Where a name must become text (a link, a URL, an environment value handed on), it is taken with `to_str` and a name that is not Unicode is left out with a warning, or refused; a link never names a file under U+FFFD. A name shown to a person goes through `Path::display`.
- A link md2html makes to a file from the file's names (`--nav`, the language versions and a translation's original, the downloads, the shared glossary's and the PDF's `file:` URLs, the address `--serve` names) writes each name as one segment of the URL, escaped by one rule (`text::PathUrl`, RFC 3986: letters, digits, `-._~`, the sub-delimiters and `@` kept, every other byte `%XX`, `/` and `:` included). A name holding a space, `#`, `?`, `%` or a letter beyond ASCII still names its file, and a name that only looks escaped (`a%20b.md`) is not read as another. The server decodes a request with the exact reverse (`text::percent_decode`) and finds a page by its names. A link the author wrote is the author's URL and is kept as written.
- `scripts/ci/anti_pattern_gate.sh` refuses `String::from_utf8_lossy`, `String::from_utf16_lossy` and `to_string_lossy` in the production code of every crate of the workspace, with no marker that lets one through. It is gate `anti-pattern` (every commit, push and pull request, and `just go`), and gate `gate-tests` plants each forbidden call to prove it is refused.

Checked by: the gate; `names_that_are_not_unicode_are_taken_as_they_are` (Linux, whose file systems take any bytes, and Windows, whose NTFS takes a surrogate without its pair) and `an_output_ending_in_a_separator_is_a_folder_whatever_its_bytes` in `src/cli/tests.rs`, both with a name that is not Unicode on each system (`tests::not_unicode`); `a_page_whose_name_is_not_unicode_says_no_link_can_name_it` in `src/tests/files.rs`; `a_name_is_one_segment_and_reads_back` in `src/text/url.rs` (every byte, and names with a space, `#`, `?`, `%`, `&`, `'`, `"`, `<`, letters beyond ASCII and `a%20b`), the tests of each place that links a file by its names (`src/tests/names.rs`, `a_link_between_pages_escapes_their_names` in `src/cli/tests.rs`, `a_name_is_escaped_in_a_file_url_and_a_relative_one` in `src/system/tests.rs`, `the_glossary_link_escapes_its_name` in `src/document/glossary.rs`, `a_page_url_escapes_its_name` in `src/export/print.rs`, `a_page_with_an_awkward_name_is_served_at_its_link` in `src/serve/tests.rs`), and `a_page_linked_by_nav_is_served_at_its_link` in `crates/md2html/tests/command_line/lasting.rs`, which runs the program; `on_unix_a_backslash_is_part_of_a_name` in `src/system/tests.rs`; `a_path_that_is_not_utf8_names_no_file` in `src/serve/tests.rs`.

## 7. Failures that disappear, and panics

A failure that is dropped (`let _ =`, `.ok()`) hides a problem from the person who could fix it; a panic on hostile input lets the author of that input stop md2html. Both are covered in [panic_policy.md](panic_policy.md): failures propagate or become warnings the reader sees, and production code does not panic.
