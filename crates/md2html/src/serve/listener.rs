// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The listener's thread: it takes the connections, answers each on a
//! thread of its own, and owns those threads.
//!
//! A connection beyond the limit is told the server is busy. When the
//! server closes, every open connection is shut, so that a thread waiting
//! on a slow browser returns at once, and every thread is waited for; a
//! thread that panicked is told as a trouble, not lost.

use core::any::Any;
use core::time::Duration;
use std::io::ErrorKind;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread::{self, Scope, ScopedJoinHandle};

use super::{Site, http};

/// How many accepts in a row may fail before the server stops.
const MAX_FAILURES: u32 = 50;

/// How long to wait after an accept that failed.
pub(super) const AFTER_FAILURE: Duration = Duration::from_millis(100);

/// Takes connections until the server closes, then closes those still
/// open and waits for their threads.
///
/// # Errors
///
/// The listener failed [`MAX_FAILURES`] times in a row (no connection
/// could be taken, or no thread started for one); the last failure.
pub(super) fn accept(socket: &TcpListener, site: &Site) -> std::io::Result<()> {
    thread::scope(|scope| {
        let mut open = Connections::default();
        let mut failures = 0_u32;
        let ended = loop {
            let accepted = socket.accept();
            if site.closing() {
                break Ok(());
            }
            open.reap(site);
            let error = match accepted.and_then(|(stream, _)| open.admit(scope, stream, site)) {
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
                break Err(error);
            }
            thread::sleep(AFTER_FAILURE);
        };
        open.close(site);
        ended
    })
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

/// A connection being answered.
#[derive(Debug)]
struct Connection<'scope> {
    /// Its thread.
    thread: ScopedJoinHandle<'scope, ()>,
    /// The connection itself, to shut it when the server closes.
    stream: TcpStream,
}

impl Connection<'_> {
    /// Waits for its thread; one that panicked is told as a trouble.
    fn join(self, site: &Site) {
        if let Err(payload) = self.thread.join() {
            site.trouble(format!(
                "a connection's thread stopped: {}",
                panic_message(&*payload)
            ));
        }
    }
}

/// The connections being answered, each on a thread the listener's thread
/// owns.
#[derive(Debug, Default)]
struct Connections<'scope> {
    /// Those not yet waited for.
    open: Vec<Connection<'scope>>,
}

impl<'scope> Connections<'scope> {
    /// Waits for the connections whose thread has ended.
    fn reap(&mut self, site: &Site) {
        let (ended, open): (Vec<_>, Vec<_>) = core::mem::take(&mut self.open)
            .into_iter()
            .partition(|connection| connection.thread.is_finished());
        self.open = open;
        for connection in ended {
            connection.join(site);
        }
    }

    /// Answers `stream` on a thread of its own, or tells the browser the
    /// server is busy when every place is taken.
    ///
    /// # Errors
    ///
    /// The connection could not be kept, or its thread not started.
    fn admit<'env>(
        &mut self,
        scope: &'scope Scope<'scope, 'env>,
        stream: TcpStream,
        site: &'env Site,
    ) -> std::io::Result<()> {
        if self.open.len() >= site.limits.connections {
            if let Err(error) = http::busy(&stream) {
                site.trouble(format!("a request from the browser failed: {error}"));
            }
            return Ok(());
        }
        let kept = stream.try_clone()?;
        let thread = thread::Builder::new()
            .name(String::from("md2html-answer"))
            .spawn_scoped(scope, move || answer(&stream, site))?;
        self.open.push(Connection {
            thread,
            stream: kept,
        });
        Ok(())
    }

    /// Shuts every connection, so that its thread returns, and waits for
    /// every thread.
    fn close(self, site: &Site) {
        for connection in &self.open {
            // A connection the browser has let go has nothing left to shut.
            if let Err(error) = connection.stream.shutdown(Shutdown::Both) {
                if error.kind() != ErrorKind::NotConnected {
                    site.trouble(format!("a connection could not be closed: {error}"));
                }
            }
        }
        for connection in self.open {
            connection.join(site);
        }
    }
}

/// Answers one connection on its thread. What goes wrong once the server
/// is closing is the closing's doing, not a trouble.
fn answer(stream: &TcpStream, site: &Site) {
    if let Err(error) = http::connection(stream, site) {
        if !site.closing() {
            site.trouble(format!("a request from the browser failed: {error}"));
        }
    }
}

/// The message a panic carried (`panic!("...")` gives a `&str` or a
/// `String`); empty for any other payload.
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or_default()
}
