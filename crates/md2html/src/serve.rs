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
//! Every thread has an owner that waits for it: [`run`] owns the
//! listener's thread, which owns one thread per connection
//! ([`listener`]). The server holds at most [`Limits::SERVE`]'s
//! number of connections and tells the rest it is busy; each connection is
//! bounded in time and in what it can make the server read. It runs until
//! the Markdown inputs are gone or its listener fails; then it closes every
//! connection and waits for every thread. Ctrl-C ends the process, and
//! with it every connection: the server writes nothing to disk, so nothing
//! is left half done.

mod http;
mod listener;
mod request;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeMap;
use core::net::SocketAddr;
use core::time::Duration;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ScopedJoinHandle};

use crate::report::{Exit, Failure, Reporter};
use crate::text::PathUrl;
use crate::watch::{Change, Watch};

/// How many ports, from the one asked for, are tried.
const PORTS_TRIED: u16 = 20;

/// How long a connection made to wake the listener's thread may take.
const WAKE_TIME: Duration = Duration::from_secs(1);

/// How many connection troubles are kept until they are told.
const MAX_TROUBLES: usize = 100;

/// The pages, by URL path.
type Pages = BTreeMap<String, String>;

/// What the server gives a browser, at most.
#[derive(Clone, Copy, Debug)]
struct Limits {
    /// How many connections are served at once.
    connections: usize,
    /// How long a request has to arrive, whole.
    request_time: Duration,
    /// How long a write may wait for the browser to read.
    write_time: Duration,
    /// How long a reload signal lasts; the page then asks for a new one.
    stream_life: Duration,
    /// How often a reload signal says it is still there.
    keep_alive: Duration,
    /// How long, after a refusal, what the browser still sends is read and
    /// thrown away, so that the refusal reaches it.
    linger: Duration,
}

impl Limits {
    /// The server's own.
    const SERVE: Self = Self {
        connections: 64,
        request_time: Duration::from_secs(10),
        write_time: Duration::from_secs(10),
        stream_life: Duration::from_secs(600),
        keep_alive: Duration::from_secs(15),
        linger: Duration::from_secs(1),
    };
}

/// The pages and how many times they have been rendered.
#[derive(Debug)]
struct State {
    /// URL path to page.
    pages: Pages,
    /// Bumped on every render; the pages' reload signal.
    generation: u64,
    /// Whether the server is closing: no connection is taken any more, and
    /// every reload signal ends.
    closing: bool,
}

/// What a reload signal says next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    /// Nothing new: that it is still there.
    Same,
    /// The pages were rendered again: the render's number.
    Render(u64),
    /// The server is closing: nothing; the signal ends.
    Closed,
}

/// What the server's threads share. It lives on [`run`]'s stack: every
/// thread that borrows it ends before it does.
#[derive(Debug)]
struct Site {
    /// The pages.
    state: Mutex<State>,
    /// Told whenever the pages are rendered again, and when the server
    /// closes.
    signal: Condvar,
    /// The folder served, links resolved.
    root: PathBuf,
    /// The port listened on, for the names this machine answers to.
    port: u16,
    /// What the server gives a browser.
    limits: Limits,
    /// What went wrong with connections, until it is told to the person
    /// running md2html.
    troubles: Mutex<Vec<String>>,
}

impl Site {
    /// A site serving `pages` from `root` on `port`, within `limits`.
    const fn new(root: PathBuf, port: u16, pages: Pages, limits: Limits) -> Self {
        Self {
            state: Mutex::new(State {
                pages,
                generation: 1,
                closing: false,
            }),
            signal: Condvar::new(),
            root,
            port,
            limits,
            troubles: Mutex::new(Vec::new()),
        }
    }

    /// The pages, locked. A thread that stopped while holding them left
    /// them whole: they are replaced in one assignment, and the other
    /// fields are numbers set in one.
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
    fn trouble(&self, message: String) {
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

    /// Closes the server: no connection is taken any more, and every
    /// reload signal is told to end.
    fn close(&self) {
        self.state().closing = true;
        self.signal.notify_all();
    }

    /// Whether the server is closing.
    fn closing(&self) -> bool {
        self.state().closing
    }

    /// What a reload signal that last said render `seen` says next: a new
    /// render as soon as there is one, that the server is closing as soon
    /// as it is, or after `wait` that nothing changed.
    fn next_signal(&self, seen: u64, wait: Duration) -> Signal {
        let (state, _) = self
            .signal
            .wait_timeout_while(self.state(), wait, |state| {
                state.generation == seen && !state.closing
            })
            .unwrap_or_else(PoisonError::into_inner);
        let (generation, closing) = (state.generation, state.closing);
        drop(state);
        if closing {
            Signal::Closed
        } else if generation == seen {
            Signal::Same
        } else {
            Signal::Render(generation)
        }
    }
}

/// The path the server answers a page at, from the page's path relative to
/// the folder served; none when that is not Unicode.
///
/// It is `/` and the path with the system's separator as `/` (elsewhere
/// `\` is part of a name), as a request's path reads once its escapes are
/// decoded. A link to the page escapes each name ([`PathUrl`]); the server
/// decodes the request, the exact reverse, and finds the page by this text.
/// A URL names a page by text, so a path that is not Unicode has none.
pub(crate) fn page_path(rel: &Path) -> Option<String> {
    rel.to_str()
        .map(|text| format!("/{}", text.replace(std::path::MAIN_SEPARATOR, "/")))
}

/// Serves `render()`'s pages from `root` on 127.0.0.1, from `port` up,
/// rendering again when one of `watch`'s files changes.
///
/// # Errors
///
/// No port is free ([`Exit::Unavailable`]), the server's thread cannot
/// start or its listener fails ([`Exit::Io`]), the server's thread stopped
/// unexpectedly ([`Exit::Software`]), or the inputs or their folder are
/// gone ([`Exit::NoInput`]).
pub(crate) fn run(
    root: &Path,
    port: u16,
    mut watch: Watch,
    mut render: impl FnMut() -> Pages,
    reporter: &Reporter,
) -> Result<(), Failure> {
    let (listener, bound) = bind(port)?;
    // Every file served is judged by where it really is against this
    // folder; a folder that cannot be resolved is gone, not taken as
    // written (against which nothing would ever be found inside).
    let real_root = std::fs::canonicalize(root).map_err(|error| {
        Failure::new(
            Exit::NoInput,
            format!("the folder {} cannot be read: {error}", root.display()),
        )
    })?;
    let site = Site::new(real_root, bound, render(), Limits::SERVE);
    // The first page's path as a browser asks for it: each name escaped.
    let first = PathUrl::relative(site.first_page().split('/').map(str::as_bytes));
    reporter.note(format_args!(
        "md2html: serving http://127.0.0.1:{bound}{} (this computer only); pages reload when the Markdown changes; Ctrl-C to stop",
        first.as_str()
    ));
    let tell = || {
        for message in site.troubles() {
            reporter.note(format_args!("md2html: {message}"));
        }
    };
    let served = serve(&listener, &site, || {
        let change = watch.wait();
        if change == Change::Changed {
            site.publish(render());
        }
        tell();
        change == Change::Gone
    });
    tell();
    served?;
    Err(Failure::new(
        Exit::NoInput,
        String::from("none of the inputs has been there for a while; the server stopped"),
    ))
}

/// Serves `site` on `socket` until `done` says the run is over, or the
/// listener fails; then closes every connection and waits for every
/// thread.
///
/// `done` runs on the calling thread, again and again while the server
/// runs: it is where the run watches its files and renders, and it waits
/// a while each time.
///
/// # Errors
///
/// The listener's thread could not start or the listener failed
/// ([`Exit::Io`]); the listener's thread stopped unexpectedly
/// ([`Exit::Software`]).
fn serve(socket: &TcpListener, site: &Site, mut done: impl FnMut() -> bool) -> Result<(), Failure> {
    let not_started = |error: std::io::Error| {
        Failure::new(Exit::Io, format!("the server could not start: {error}"))
    };
    let address = socket.local_addr().map_err(not_started)?;
    thread::scope(|scope| {
        let accepting = thread::Builder::new()
            .name(String::from("md2html-serve"))
            .spawn_scoped(scope, || listener::accept(socket, site))
            .map_err(not_started)?;
        let closing = Closing {
            site,
            address,
            accepting: &accepting,
        };
        while !accepting.is_finished() && !done() {}
        drop(closing);
        match accepting.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(Failure::new(
                Exit::Io,
                format!("the server stopped: {error}"),
            )),
            Err(_) => Err(Failure::new(
                Exit::Software,
                String::from("the server stopped unexpectedly"),
            )),
        }
    })
}

/// Closes the server when dropped: when the run is over, and when the
/// run panics, so that the wait for the listener's thread, which a thread
/// scope makes even then, ends.
struct Closing<'serve, 'scope> {
    /// The site to close.
    site: &'serve Site,
    /// The listener's address, to wake its thread.
    address: SocketAddr,
    /// The listener's thread.
    accepting: &'serve ScopedJoinHandle<'scope, std::io::Result<()>>,
}

impl Drop for Closing<'_, '_> {
    fn drop(&mut self) {
        self.site.close();
        // The listener's thread waits for a connection: one of its own
        // wakes it, and it sees the server closing.
        while !self.accepting.is_finished()
            && TcpStream::connect_timeout(&self.address, WAKE_TIME).is_err()
        {
            thread::sleep(listener::AFTER_FAILURE);
        }
    }
}

/// A listener on 127.0.0.1, on the first free port from `port` up, and
/// the port it listens on.
///
/// Port 0 asks the system for a free one: the port is then the one the
/// system gave, which the server answers for and names.
///
/// # Errors
///
/// As [`first_free`]; [`Exit::Io`] when the listener cannot say its port.
fn bind(port: u16) -> Result<(TcpListener, u16), Failure> {
    let listener = first_free(port, |tried| TcpListener::bind(("127.0.0.1", tried)))?;
    let bound = listener
        .local_addr()
        .map_err(|error| Failure::new(Exit::Io, format!("the server could not start: {error}")))?;
    Ok((listener, bound.port()))
}

/// What `open` gives for the first of the [`PORTS_TRIED`] ports from
/// `port` up that it can open.
///
/// # Errors
///
/// With [`Exit::Unavailable`] when it can open none of them:
/// `no free port from <port> on 127.0.0.1 (port <last>: <reason>)`, the
/// reason the last one was refused (in use, or not allowed).
fn first_free<Listener>(
    port: u16,
    mut open: impl FnMut(u16) -> std::io::Result<Listener>,
) -> Result<Listener, Failure> {
    let mut refused = None;
    for tried in port..port.saturating_add(PORTS_TRIED) {
        match open(tried) {
            Ok(listener) => return Ok(listener),
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
