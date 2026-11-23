// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html --serve`: the pages served on this computer while they are
//! written.
//!
//! Each save renders them again and every open page reloads,
//! keeping the reader's place (`page/live.js`). Nothing leaves the machine:
//! the server listens on the loopback address only, answers only to the
//! names of this machine (no DNS rebinding), serves only files inside the
//! document's folder, and only to GET and HEAD.
//!
//! It holds at most [`MAX_CONNECTIONS`] connections at once and answers
//! the rest that it is busy; a request has [`http::HEAD_TIME`] to arrive,
//! and a page's reload signal ends after [`http::STREAM_LIFE`], when the
//! page asks for it again by itself. It runs until the Markdown inputs are
//! gone, or its listener fails.

mod http;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use crate::report::{Exit, Failure, Reporter};
use crate::watch::{Change, Watch};

/// How many connections are served at once.
const MAX_CONNECTIONS: usize = 64;

/// How many ports, from the one asked for, are tried.
const PORTS_TRIED: u16 = 20;

/// How many accepts in a row may fail before the server stops.
const MAX_FAILURES: u32 = 50;

/// How long to wait after an accept that failed.
const AFTER_FAILURE: Duration = Duration::from_millis(100);

/// How many connection troubles are kept until they are told.
const MAX_TROUBLES: usize = 100;

/// The pages, by URL path.
type Pages = BTreeMap<String, String>;

/// The pages and how many times they have been rendered.
#[derive(Debug)]
struct State {
    /// URL path to page.
    pages: Pages,
    /// Bumped on every render; the pages' reload signal.
    generation: u64,
}

/// What the server's threads share.
#[derive(Debug)]
struct Site {
    /// The pages.
    state: Mutex<State>,
    /// Told whenever the pages are rendered again.
    signal: Condvar,
    /// The folder served, links resolved.
    root: PathBuf,
    /// The port listened on, for the names this machine answers to.
    port: u16,
    /// How many connections are being served.
    open: AtomicUsize,
    /// What went wrong with connections, until it is told to the person
    /// running md2html.
    troubles: Mutex<Vec<String>>,
}

impl Site {
    /// A site serving `pages` from `root` on `port`.
    const fn new(root: PathBuf, port: u16, pages: Pages) -> Self {
        Self {
            state: Mutex::new(State {
                pages,
                generation: 1,
            }),
            signal: Condvar::new(),
            root,
            port,
            open: AtomicUsize::new(0),
            troubles: Mutex::new(Vec::new()),
        }
    }

    /// The pages, locked. A thread that stopped while holding them left
    /// them whole: they are replaced in one assignment.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The page at `path`; for `/`, the first `index.html`, else the first
    /// page, when there is one.
    fn page(&self, path: &str) -> Option<String> {
        let state = self.state();
        if path != "/" {
            return state.pages.get(path).cloned();
        }
        state
            .pages
            .iter()
            .find(|(key, _)| key.ends_with("/index.html"))
            .or_else(|| state.pages.iter().next())
            .map(|(_, page)| page.clone())
    }

    /// The first page's path.
    fn first_page(&self) -> String {
        self.state()
            .pages
            .keys()
            .next()
            .cloned()
            .unwrap_or_default()
    }

    /// Keeps what went wrong with a connection, until it is told.
    fn trouble(&self, error: &std::io::Error) {
        let message = format!("a request from the browser failed: {error}");
        let mut troubles = self.troubles.lock().unwrap_or_else(PoisonError::into_inner);
        if troubles.len() < MAX_TROUBLES {
            troubles.push(message);
        }
    }

    /// What went wrong since the last look.
    fn troubles(&self) -> Vec<String> {
        core::mem::take(&mut *self.troubles.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Puts new pages up and tells every open page.
    fn publish(&self, pages: Pages) {
        self.replace(pages);
        self.signal.notify_all();
    }

    /// Replaces the pages and counts the render; the lock is let go
    /// before the open pages are told.
    fn replace(&self, pages: Pages) {
        let mut state = self.state();
        state.pages = pages;
        state.generation = state.generation.wrapping_add(1);
    }

    /// The render's number once it is not `seen`, or after `wait` when it
    /// still is.
    fn generation_after(&self, seen: u64, wait: Duration) -> u64 {
        let (state, _) = self
            .signal
            .wait_timeout_while(self.state(), wait, |state| state.generation == seen)
            .unwrap_or_else(PoisonError::into_inner);
        state.generation
    }
}

/// One of the [`MAX_CONNECTIONS`] places, held while a connection is served.
#[derive(Debug)]
struct Slot {
    /// The site it is a place of.
    site: Arc<Site>,
}

impl Slot {
    /// A free place, if there is one. The place is counted before it is
    /// looked at, so two connections cannot both take the last one; a
    /// place not taken is given back when the slot is dropped.
    fn take(site: &Arc<Site>) -> Option<Self> {
        let before = site.open.fetch_add(1, Ordering::AcqRel);
        let slot = Self {
            site: Arc::clone(site),
        };
        (before < MAX_CONNECTIONS).then_some(slot)
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.site.open.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Serves `render()`'s pages from `root` on 127.0.0.1, from `port` up,
/// rendering again when one of `watch`'s files changes.
///
/// # Errors
///
/// No port is free ([`Exit::Unavailable`]), the server's thread cannot
/// start or its listener fails ([`Exit::Io`]), or the inputs are gone
/// ([`Exit::NoInput`]).
pub(crate) fn run(
    root: &Path,
    port: u16,
    mut watch: Watch,
    mut render: impl FnMut() -> Pages,
    reporter: &Reporter,
) -> Result<(), Failure> {
    let (listener, bound) = bind(port)?;
    let real_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let site = Arc::new(Site::new(real_root, bound, render()));
    reporter.note(format_args!(
        "md2html: serving http://127.0.0.1:{bound}{} (this computer only); pages reload when the Markdown changes; Ctrl-C to stop",
        site.first_page()
    ));
    let server = Arc::clone(&site);
    let accepting = std::thread::Builder::new()
        .name(String::from("md2html-serve"))
        .spawn(move || accept(&listener, &server))
        .map_err(|error| Failure::new(Exit::Io, format!("the server could not start: {error}")))?;
    let mut change = Change::Same;
    while change != Change::Gone && !accepting.is_finished() {
        change = watch.wait();
        if change == Change::Changed {
            site.publish(render());
        }
        for message in site.troubles() {
            reporter.note(format_args!("md2html: {message}"));
        }
    }
    if change == Change::Gone {
        return Err(Failure::new(
            Exit::NoInput,
            String::from("none of the inputs has been there for a while; the server stopped"),
        ));
    }
    Err(accepting.join().map_or_else(
        |_| {
            Failure::new(
                Exit::Software,
                String::from("the server stopped unexpectedly"),
            )
        },
        |error| Failure::new(Exit::Io, format!("the server stopped: {error}")),
    ))
}

/// A listener on 127.0.0.1, on the first free port from `port` up.
///
/// # Errors
///
/// As [`first_free`].
fn bind(port: u16) -> Result<(TcpListener, u16), Failure> {
    first_free(port, |tried| TcpListener::bind(("127.0.0.1", tried)))
}

/// What `open` gives for the first of the [`PORTS_TRIED`] ports from
/// `port` up that it can open, and that port.
///
/// # Errors
///
/// With [`Exit::Unavailable`] when it can open none of them:
/// `no free port from <port> on 127.0.0.1 (port <last>: <reason>)`, the
/// reason the last one was refused (in use, or not allowed).
fn first_free<Listener>(
    port: u16,
    mut open: impl FnMut(u16) -> std::io::Result<Listener>,
) -> Result<(Listener, u16), Failure> {
    let mut refused = None;
    for tried in port..port.saturating_add(PORTS_TRIED) {
        match open(tried) {
            Ok(listener) => return Ok((listener, tried)),
            Err(error) => refused = Some((tried, error)),
        }
    }
    let why = refused.map_or_else(String::new, |(last, error)| {
        format!(" (port {last}: {error})")
    });
    Err(Failure::new(
        Exit::Unavailable,
        format!("no free port from {port} on 127.0.0.1{why}"),
    ))
}

/// Accepts connections until the listener fails; the reason it failed.
fn accept(listener: &TcpListener, site: &Arc<Site>) -> std::io::Error {
    let mut failures = 0_u32;
    for incoming in listener.incoming() {
        let error = match incoming.and_then(|stream| admit(stream, site)) {
            Ok(()) => {
                failures = 0;
                continue;
            }
            Err(error) => error,
        };
        if passing(error.kind()) {
            continue;
        }
        failures = failures.saturating_add(1);
        if failures >= MAX_FAILURES {
            return error;
        }
        std::thread::sleep(AFTER_FAILURE);
    }
    std::io::Error::other("the listener closed")
}

/// Whether an accept failed for that one connection only.
const fn passing(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionReset
            | ErrorKind::Interrupted
            | ErrorKind::WouldBlock
            | ErrorKind::TimedOut
    )
}

/// Serves one connection on a thread of its own, or says the server is
/// busy when every place is taken.
///
/// # Errors
///
/// The thread could not be started.
fn admit(stream: TcpStream, site: &Arc<Site>) -> std::io::Result<()> {
    let Some(slot) = Slot::take(site) else {
        if let Err(error) = http::busy(&stream) {
            site.trouble(&error);
        }
        return Ok(());
    };
    std::thread::Builder::new()
        .name(String::from("md2html-answer"))
        .spawn(move || {
            if let Err(error) = http::connection(&stream, &slot.site) {
                slot.site.trouble(&error);
            }
        })
        .map(|_| ())
}
