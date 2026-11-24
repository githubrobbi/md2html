<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Unsafe code, foreign functions and the operating systems

For whoever is tempted by `unsafe`, a foreign function or a `cfg` on a system: why there are none of the first two and one module with the third, and where each difference between Linux, macOS and Windows is handled and tested.

## Unsafe code and foreign functions

There are none. `unsafe_code = "forbid"` stands in `[workspace.lints.rust]` of the root `Cargo.toml`, and every crate takes the workspace's lints, so no crate and no item can allow it back: `forbid`, unlike `deny`, cannot be lifted where it is used. No crate declares an `extern` block, links a library (`#[link]`) or exports a symbol (`no_mangle`), and none depends on a crate for system calls. What md2html needs of the operating system, the standard library gives safely; what it does not give (a signal handler for Ctrl-C, a file lock) md2html does without, and this page says how where it matters (`concurrency_policy.md`, "How it stops").

A crate that one day needs `unsafe` or a foreign function starts with a policy here, in the same pull request: why it cannot do without, the one small module the unsafe code is confined to behind a safe interface, the invariants each block relies on (`// SAFETY:` comments, which `undocumented_unsafe_blocks` already requires), and the tests, Miri included, that hold them. The workspace lints for that day are already set (`unsafe_op_in_unsafe_fn`, `multiple_unsafe_ops_per_block`, `raw_borrows_via_references`).

## The operating systems

md2html runs on Linux, macOS and Windows. Where the three differ, the standard library says how, and md2html uses what it says instead of its own rule: `Path::components` reads a drive or a share only on Windows, `std::path::is_separator` and `MAIN_SEPARATOR` are `\` as well as `/` only there, `std::env::consts::EXE_SUFFIX` is `.exe` only there, `split_paths` splits `PATH` on the system's character. The few rules that are md2html's own are in one module, `system` (where the home folder is, a path as a `file:` URL, which relative paths stay below their folder, how standard output is written), and in the list of places Chrome is installed (`export/print.rs`). Production code has a `cfg` on a system in that module only, for the one rule the standard library cannot give: standard output (below).

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
| Standard output that refuses a write | `EBADF` (opened only for reading), which std's `Stdout` of Rust 1.85 to 1.100 takes for a success | the same | `ERROR_ACCESS_DENIED`, which std reports; a console takes text only through std, as UTF-16 | `system::StandardOutput`: a handle of md2html's own, a terminal through std (below) | `a_standard_output_that_refuses_writes_ends_with_74_or_73`, `standard_output_takes_a_pipe_a_file_or_the_null_device` (every system, 1.85 included) |
| Standard output closed before the start | std opens `/dev/null` in its place before `main` | the same | no handle: duplicating it fails | the run ends as one into `/dev/null` does (`command-line.md`, the exit codes) | `a_closed_standard_output_is_the_null_device` (Unix) |

md2html opens no browser and starts no program but Chrome.

## Standard output

md2html writes its standard output (the page with `-o -` or from standard input, the summary, `nothing to fix`, the help, the version) through one type, `system::StandardOutput` (`crates/md2html/src/system/output.rs`), and nothing else names the standard library's standard output.

- **The rule.** Each write goes through a handle of md2html's own on what the process was given: on Linux and macOS its descriptor (`AsFd`, `try_clone_to_owned`, a `File`), on Windows its handle (`AsHandle`, the same). The bytes are those std would write. A terminal is written through std's `Stdout`, since Windows' console takes text only through it (std writes UTF-16 there) and a terminal is never a file opened for reading only; a system that is neither Unix nor Windows writes through std too.
- **Why.** std's `Stdout` takes the error `EBADF` for a success, and a standard output opened only for reading gives `EBADF`: md2html built with Rust 1.85, md2html's minimum, or any stable release up to 1.100, wrote a page there, lost it and ended with 0. The nightly releases from 2026-11-21 report the error; 1.101, in beta on 2026-11-24, does not. Through its own handle md2html sees the error whatever Rust built it and ends with 74 (73 for the page), as `docs/command-line.md` promises; the `msrv` gate and CI's `msrv` job run the test on 1.85.
- **What it cannot see.** A standard output closed before md2html starts (`>&-`) is opened again on `/dev/null`, for reading and writing, by std before `main` runs, so that no file md2html opens takes its number. Nothing tells it from a standard output a parent opened on `/dev/null` the same way (Python's `subprocess.DEVNULL` does), so such a run ends as one into `/dev/null` does, with 0; a test holds that, and fails the day std changes it.
- **Where the `cfg` is.** In `crates/md2html/src/system/output.rs` alone: the duplicate is made by `std::os::fd` on Unix and `std::os::windows::io` on Windows, which exist only there. `only_system_names_standard_output_and_the_systems` (`crates/md2html/src/layer_tests.rs`) fails on `io::stdout`, `stdout()` or a `cfg` other than `cfg(test)` in any other production file. `just lint-pre-push` lints the Windows branch through `cargo xwin`; `ci.yml` runs the tests on all three systems.
- **When it can go.** Once md2html's minimum Rust is a stable release whose standard library reports the error itself (Rust 1.102, due around 4 February 2027; it may rise to any released stable Rust by the rule of `release_policy.md`, "The minimum Rust"), std's `Stdout` can be written directly again: `StandardOutput` keeps its interface and loses the duplicate, and the `cfg` and this section go. The tests stay.

## Where each system is tested

`ci.yml` runs md2html's tests on Linux, macOS and Windows on every pull request, so a test marked `cfg(windows)` or `cfg(unix)` runs on the system whose rule it holds. The tooling's tests run on Linux (`pr-fast.yml`); the tooling has no rule of its own for a system (it takes `EXE_SUFFIX` and `consts::OS` from std), so nothing in it waits for a Windows run. `pr-fast.yml` `windows-lint` and `just lint-pre-push` (through `cargo xwin`) lint every `cfg(windows)` line from any machine.

## The rule for new code

- No `cfg` on a system in production code outside `system`. Take the standard library's answer (`Path::components`, `is_separator`, `EXE_SUFFIX`, `split_paths`); a rule md2html adds goes in `system`, as a function of its inputs, so every system can test it. A `cfg` in `system` is for what only a system's own module of std offers (the handle of standard output), says why, and when it can go.
- Standard output is written through `system::StandardOutput`, never `std::io::stdout` (`print!` and `println!` are denied by `print_stdout` already).
- A decision about a path is made on its parts as the system reads them, and a decision about what may be read on the path the file system resolves, never on the text.
- Where the systems differ, a test holds each system's rule on that system: `cfg(windows)` for Windows' and `cfg(unix)` for Linux's and macOS's, beside a test that holds what is the same everywhere.
- Nothing that differs by system goes in a module the weekly mutation run covers (`scripts/ci/mutants-scope.txt`): it runs on Linux, where a Windows branch would be a mutant no test can catch.
