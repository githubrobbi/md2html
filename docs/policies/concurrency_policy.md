<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Concurrency

md2html runs on one thread. Rendering, `--check`, `--watch` (a loop on the main thread that looks at the inputs' times of change a few times a second, `watch.rs`) and the EPUB writer share nothing. `--pdf` starts Chrome as a child process and waits for it with a time limit of 90 seconds, then kills it (`export/print.rs`). The one place with threads is the local server.

There is no async code in the workspace and no async runtime, in md2html or in the tooling.

## The server

`md2html --serve` (`serve.rs`) renders and watches on the main thread, accepts connections on one thread of its own, and answers each connection on a thread of its own.

- **What is shared.** One `Arc<Site>`: the rendered pages and a render count in a `Mutex<State>`, a `Condvar` that the reload signals wait on, the number of open connections in an `AtomicUsize`, and the connection troubles not yet told, in a `Mutex<Vec<String>>`. Nothing else crosses threads.
- **A lock is held for a copy or a swap, never across work.** The main thread renders first and takes the lock only to put the new pages in, in one assignment, then wakes the reload signals after letting go (`Site::publish`). A request takes the lock to copy one page out and writes it to the socket after letting go. A reload signal waits on the condition variable for at most 15 seconds at a time and lets go of the lock before it writes.
- **A poisoned lock is not a second panic.** A thread that stopped while holding the pages left them whole, because they are replaced in one assignment; the next thread takes them as they are (`PoisonError::into_inner`). No `lock()` result is unwrapped.
- **The number of threads is bounded.** At most 64 connections are served at once (`MAX_CONNECTIONS`). A place is counted before it is looked at, so two connections cannot both take the last one, and it is given back when the connection's `Slot` is dropped. A connection beyond the limit is told the server is busy and closed.
- **Every connection is bounded in time and in what it can make the server read.** A request has 10 seconds to arrive whole, however slowly it is sent; a write may wait 10 seconds for the browser; the request line and each header are at most 8 KB, and at most 64 headers are read. A reload signal ends after 10 minutes, and the page asks for a new one by itself.

Enforced by: `significant_drop_tightening` and `significant_drop_in_scrutinee` (a lock guard lives no longer than its use), `mutex_atomic`, `rc_mutex` and `clone_on_ref_ptr` in `Cargo.toml`, at deny; `unwrap_used` and `expect_used` for the lock results. The limits are checked by `connections_beyond_the_limit_are_turned_away`, `a_request_is_read_within_its_limits` and `the_reload_signal_says_the_render_and_ends` in `src/serve/tests.rs`. `tests/browser/live.js` (CI job `browser`) runs the server and checks that a saved change reloads the page within two seconds, keeping the reader's place, and that a path outside the document's folder is not served. What the server promises is in [SECURITY.md](../../SECURITY.md), guarantee 5.

## The tooling

The pipeline behind `just go` and `just ship` (`scripts/ci-pipeline`) is plain functions, with no async code and no runtime. It runs `cargo` and `git` as child processes, each with a time limit (`src/child.rs`): the child's output is read on two scoped threads while its exit is polled, and past the limit it is killed. The parallel validation stage runs its commands on a fixed number of scoped worker threads, which take the next command from a shared atomic index and start no new one after the first failure (`src/exec.rs`).

Scoped threads are joined before their scope ends, so every thread has an owner that waits for it, and a thread that panics is reported as an error by the code that joins it, not lost. Shared state between them is atomics, not locks.

The lints for async code stay at deny, so the rule holds the day async code arrives: a lock guard, a `RefCell` borrow or an `Rc` does not live across an `.await` (`await_holding_lock`, `await_holding_refcell_ref`, `await_holding_invalid_type`), and a function that never awaits is not `async` (`unused_async`).
