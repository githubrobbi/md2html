<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Allocation

## The rule

A page is built by allocating, and that is fine. A copy that nothing needs is not: borrow before cloning, and write into the output instead of building strings to throw away.

## Why

md2html turns a document of up to 64 MB into one `String`, passing it through the block parser, the inline parser, the renderer and, for raw HTML, the sanitizer. A needless copy per line or per inline element is paid once for every piece of the document at every step. The cost is invisible in a small file and dominant in a large one, and the lints below find most of it before it is written.

Error messages and warnings are the exception that needs no thought: they are made once, when something has gone wrong, and may allocate freely.

## Enforced by

`Cargo.toml`, at deny: `redundant_clone`, `implicit_clone`, `clone_on_ref_ptr`, `cloned_instead_of_copied`, `map_clone`, `assigning_clones`, `str_to_string`, `inefficient_to_string`, `string_add`, `string_add_assign`, `format_push_string`, `needless_collect`, `needless_pass_by_value`, `large_types_passed_by_value`, `trivially_copy_pass_by_ref`, `large_stack_arrays`, `large_enum_variant`, `rc_buffer`, `rc_mutex`, `or_fun_call`, `read_zero_byte_vec`; with the `perf` lints Clippy runs by default. Gates `lint-prod`, `lint-tests`, `lint-ci` ([lint-posture.md](lint-posture.md)).

## The shapes that avoid the copy

- **Write into the output.** The page is one `String`, handed down as `&mut String`. Append with `push_str`, or with `put!` (in `text.rs`) for formatted text, not `format!` and then push (`format_push_string`), and not `+` (`string_add`). `put!` gives back no result, by the owner's decision: appending to a `String` cannot fail ([lint-posture.md](lint-posture.md), "The rule").
- **Borrow the input.** Take `&str` and `&[T]`; give back pieces of the input rather than copies. `Scan` and `Cursor` in `crate::scan` return `&'text str` and `&'items [Item]` that point into what they read. The page model (`model.rs`) keeps a leaf's inline text as it was written, and the inline pass reads it once every definition is known.
- **Take ownership only to keep.** A function that stores or consumes a value takes it by value; one that only reads it takes a reference (`needless_pass_by_value`).
- **Size it once.** When the length is known, `String::with_capacity` (as `text::esc` does). A read with a limit (`input::read_limited`) does not size its buffer from what the file system says a file holds: a pipe or a device says what it likes.
- **Reuse a buffer.** `clone_from` or `clone_into` over an existing `String` keeps its memory (`assigning_clones`); the server reads the `Host` header this way (`serve/http.rs`).
- **Say what a clone costs.** Shared state is cloned with `Arc::clone(site)`, which reads as a reference count, not as `site.clone()`, which reads as a deep copy (`clone_on_ref_ptr`); the server hands its `Arc<Site>` to each connection this way (`serve.rs`). Small `Copy` values are copied with `.copied()`.
- **Convert plainly.** `&str` to `String` is `to_owned()` (`str_to_string`).
