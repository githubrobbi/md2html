<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Unsafe code, foreign functions and the operating systems

## Unsafe code and foreign functions

There are none. `unsafe_code = "forbid"` stands in `[workspace.lints.rust]` of the root `Cargo.toml`, and every crate takes the workspace's lints, so no crate and no item can allow it back: `forbid`, unlike `deny`, cannot be lifted where it is used. No crate declares an `extern` block, links a library (`#[link]`) or exports a symbol (`no_mangle`), and none depends on a crate for system calls. What md2html needs of the operating system, the standard library gives safely; what it does not give (a signal handler for Ctrl-C, a file lock) md2html does without, and this page says how where it matters (`concurrency_policy.md`, "How it stops").

A crate that one day needs `unsafe` or a foreign function starts with a policy here, in the same pull request: why it cannot do without, the one small module the unsafe code is confined to behind a safe interface, the invariants each block relies on (`// SAFETY:` comments, which `undocumented_unsafe_blocks` already requires), and the tests, Miri included, that hold them. The workspace lints for that day are already set (`unsafe_op_in_unsafe_fn`, `multiple_unsafe_ops_per_block`, `raw_borrows_via_references`).

## The operating systems

md2html runs on Linux, macOS and Windows, and its production code has no `cfg` on a system. Where the three differ, the standard library says how, and md2html uses what it says instead of its own rule: `Path::components` reads a drive or a share only on Windows, `std::path::is_separator` and `MAIN_SEPARATOR` are `\` as well as `/` only there, `std::env::consts::EXE_SUFFIX` is `.exe` only there, `split_paths` splits `PATH` on the system's character. The few rules that are md2html's own are in one module, `system` (where the home folder is, a path as a `file:` URL, which relative paths stay below their folder), and in the list of places Chrome is installed (`export/print.rs`).

| What differs | Linux | macOS | Windows | Where | Held by |
| --- | --- | --- | --- | --- | --- |
| A file name | any bytes | UTF-8 (APFS) | UTF-16, a surrogate may stand alone | decided on the `OsStr` (`hostile_input_policy.md`, rule 6) | `names_that_are_not_unicode_are_taken_as_they_are` (Linux, Windows), the tests built on `tests::not_unicode` (every system) |
| A separator in a path | `/` | `/` | `/` and `\` | std's; links and served URLs use `/` (`cli/files.rs` `relative`, `cli/live.rs`), `-o` ending in one is a folder (`cli/plan.rs`) | `on_unix_a_backslash_is_part_of_a_name`, `on_windows_a_backslash_separates_and_drives_link_by_url` |
| A path that leaves a folder | `/x`, `..` | `/x`, `..` | also `\x`, `C:x`, `C:\x`, `\\server\share` | `system::file_below`, then the resolved path (`text::inside`) | `only_names_stay_below`, `on_windows_drives_and_roots_leave_the_folder`, `an_image_outside_the_folder_is_not_packed` |
| A path ending in a separator | Linux refuses to open `a.png/` | macOS too | Windows opens it as `a.png` | md2html's rule on every system: it names a folder (`system::ends_in_separator`, `file_below`); the server serves nothing under another spelling of a file's names (`a//b`, `./a`) | `a_file_is_served_only_under_its_own_names`, `an_image_asked_for_as_a_folder_is_not_packed`, `a_path_ending_in_a_separator_names_a_folder` |
| A canonical path | `/a/b` | `/a/b` | `\\?\C:\a\b`, `\\?\UNC\server\share\b` | `system::file_url` reads the verbatim forms as the plain ones | `on_windows_drives_and_shares_lead_the_url`, `a_resolved_folder_gives_a_plain_file_url` |
| A hidden file | a name starting with `.` | the same | the same; also reached by `\`, `%5C` or a short name (`GIT~1`) | `serve/http.rs`: judged on what is asked for and on where it leads | `on_windows_a_backslash_does_not_hide_a_hidden_file`, `a_link_out_of_the_folder_is_not_followed` |
| Case in names | matters | ignored (by default) | ignored | an output is compared to the input as the file system resolves both | `a_name_in_another_case_is_the_input_where_case_is_ignored` |
| Line ends in the input | LF | LF | often CRLF | read as CommonMark says; the page is written with LF | `windows_line_endings_read_as_the_same_document` |
| The home folder | `HOME` | `HOME` | `USERPROFILE` | `system::home` | `the_home_folder_is_home_else_the_user_profile` |
| Chrome for `--pdf` | on `PATH` | `/Applications`, `~/Applications`, `PATH` | under `ProgramFiles`, `ProgramFiles(x86)`, `LOCALAPPDATA`; `chrome.exe`, `msedge.exe` on `PATH` | `export/print.rs` `Places`; none found is an error that says so | `without_chrome_a_pdf_says_what_it_needs`, `installed_places_come_before_path`, `on_windows_chrome_is_found_by_its_exe_name` |
| The clock of a file | fine (ext4) | fine (APFS); 1 s on HFS+ | fine (NTFS); 2 s on FAT and exFAT | `watch.rs`: a change is the time or the size | `a_save_within_one_tick_of_a_coarse_clock_is_seen` |
| Renaming over a file | replaces it | replaces it | replaces it; refused while another program holds it open | `std::fs::rename`; the error names the file and the temporary file is removed | `a_file_is_written_whole_and_a_planted_link_is_not_followed` |
| A shut connection | wakes a blocked read and write | the same | wakes neither | `serve/`: reads wait in slices, a write ends by its limit | `concurrency_policy.md`, "How it stops" |
| A port that cannot be had | in use | in use | in use, or in a reserved range (refused, not in use) | `serve.rs` `first_free` tries the next on any refusal | `a_port_that_cannot_be_opened_is_named_with_its_reason` |

md2html opens no browser and starts no program but Chrome.

## Where each system is tested

`ci.yml` runs md2html's tests on Linux, macOS and Windows on every pull request, so a test marked `cfg(windows)` or `cfg(unix)` runs on the system whose rule it holds. The tooling's tests run on Linux (`pr-fast.yml`); the tooling has no rule of its own for a system (it takes `EXE_SUFFIX` and `consts::OS` from std), so nothing in it waits for a Windows run. `pr-fast.yml` `windows-lint` and `just lint-pre-push` (through `cargo xwin`) lint every `cfg(windows)` line from any machine.

## The rule for new code

- No `cfg` on a system in production code. Take the standard library's answer (`Path::components`, `is_separator`, `EXE_SUFFIX`, `split_paths`); a rule md2html adds goes in `system`, as a function of its inputs, so every system can test it.
- A decision about a path is made on its parts as the system reads them, and a decision about what may be read on the path the file system resolves, never on the text.
- Where the systems differ, a test holds each system's rule on that system: `cfg(windows)` for Windows' and `cfg(unix)` for Linux's and macOS's, beside a test that holds what is the same everywhere.
- Nothing that differs by system goes in `text` or `sanitize`: the weekly mutation run tests them on Linux, where a Windows branch would be a mutant no test can catch.
