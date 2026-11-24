// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The server over real connections: its refusals reach the browser, its
//! limits hold, and it stops with connections open.

use core::net::SocketAddr;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Instant;

use super::super::{Limits, serve};
use super::site_within;

/// Limits short enough to be tried in a test.
const QUICK: Limits = Limits {
    connections: 3,
    request_time: Duration::from_millis(300),
    write_time: Duration::from_secs(5),
    stream_life: Duration::from_secs(60),
    keep_alive: Duration::from_secs(60),
    linger: Duration::from_secs(1),
};

/// How long a test's client waits for an answer before it gives up.
const PATIENCE: Duration = Duration::from_secs(20);

/// What a test server left: what its client gave, how long the server took
/// to stop once asked, and the troubles it kept.
struct Served<Given> {
    /// What the client gave.
    given: Given,
    /// From asking the server to stop to its last thread ending.
    stopping: Duration,
    /// What went wrong with connections, as the person would be told.
    troubles: Vec<String>,
}

/// Serves a test site within `limits` while `client` runs against it on
/// this thread, then stops the server and waits for it.
///
/// # Panics
///
/// When the listener cannot be opened or its address read, when the site's
/// folder cannot be made or removed, or when the server fails or panics.
fn served<Given>(
    name: &str,
    limits: Limits,
    client: impl FnOnce(SocketAddr) -> Given,
) -> Served<Given> {
    let socket = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = socket.local_addr().unwrap();
    let (site, dir) = site_within(name, address.port(), limits);
    let stop = AtomicBool::new(false);
    let (given, stopping) = thread::scope(|scope| {
        let server = scope.spawn(|| {
            serve(&socket, &site, || {
                thread::sleep(Duration::from_millis(5));
                stop.load(Ordering::Acquire)
            })
        });
        let asking = Stop(&stop);
        let given = client(address);
        let asked = Instant::now();
        drop(asking);
        server.join().unwrap().unwrap();
        (given, asked.elapsed())
    });
    let troubles = site.troubles();
    std::fs::remove_dir_all(dir).unwrap();
    Served {
        given,
        stopping,
        troubles,
    }
}

/// Asks the test server to stop when dropped: when the client is done, and
/// when it panics, so that a failed test fails instead of waiting forever.
struct Stop<'flag>(&'flag AtomicBool);

impl Drop for Stop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// A connection to `address` whose reads give up after [`PATIENCE`].
///
/// # Panics
///
/// When the connection cannot be made or its time limit set.
fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(address).unwrap();
    stream.set_read_timeout(Some(PATIENCE)).unwrap();
    stream
}

/// Everything `stream` brings until the server ends it, as text.
///
/// # Panics
///
/// When reading fails or runs past [`PATIENCE`], or the answer is not
/// UTF-8.
fn rest(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// Sends `request` on a connection of its own; the whole answer.
///
/// # Panics
///
/// As [`connect`] and [`rest`], and when the request cannot be sent.
fn exchange(address: SocketAddr, request: &[u8]) -> String {
    let mut stream = connect(address);
    stream.write_all(request).unwrap();
    rest(&mut stream)
}

/// A GET of `/notes.html`, addressed to the server.
fn get(address: SocketAddr) -> Vec<u8> {
    format!(
        "GET /notes.html HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        address.port()
    )
    .into_bytes()
}

/// A refusal reaches the browser whole, even when the browser sent more
/// than was read; a good request is answered as ever.
///
/// # Panics
///
/// When a request line of 9,000 bytes is not answered 414, a line that is
/// not a request 400, a POST with a body 405, or a GET of the page with
/// the page; or when the server keeps a trouble.
#[test]
fn refusals_reach_the_browser_whole() {
    let served = served("refusals", QUICK, |address| {
        let host = format!("Host: 127.0.0.1:{}\r\n", address.port());
        let long = format!("GET /{} HTTP/1.1\r\n{host}\r\n", "a".repeat(9000));
        let post = format!(
            "POST /notes.html HTTP/1.1\r\n{host}Content-Length: 10000\r\n\r\n{}",
            "x".repeat(10_000)
        );
        [
            exchange(address, long.as_bytes()),
            exchange(address, b"GARBAGE\r\n\r\n"),
            exchange(address, post.as_bytes()),
            exchange(address, &get(address)),
        ]
    });
    let [long, garbage, post, page] = served.given;
    assert!(long.starts_with("HTTP/1.1 414 URI Too Long\r\n"), "{long}");
    assert!(long.ends_with("\r\n\r\nURI Too Long\n"), "{long}");
    assert!(
        garbage.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{garbage}"
    );
    assert!(
        post.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{post}"
    );
    assert!(page.starts_with("HTTP/1.1 200 OK\r\n"), "{page}");
    assert!(page.ends_with("\r\n\r\n<p>notes</p>"), "{page}");
    assert_eq!(served.troubles, Vec::<String>::new());
}

/// A request that does not arrive in its time is answered 408; a
/// connection that asks nothing is closed without an answer.
///
/// # Panics
///
/// When a request line with no headers after it is not answered 408 once
/// its 300 ms are over (and before [`PATIENCE`]), or a connection that
/// sends nothing gets anything but its end.
#[test]
fn a_slow_request_is_answered_408_and_a_silent_one_closed() {
    let served = served("slow", QUICK, |address| {
        let started = Instant::now();
        let slow = exchange(address, b"GET /notes.html HTTP/1.1\r\n");
        let waited = started.elapsed();
        let silent = rest(&mut connect(address));
        (slow, waited, silent)
    });
    let (slow, waited, silent) = served.given;
    assert!(
        slow.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "{slow}"
    );
    assert!(waited >= QUICK.request_time, "{waited:?}");
    assert_eq!(silent, "", "closed without an answer");
}

/// No more connections are served at once than the limit; the next is
/// told the server is busy, and a place given back is taken again.
///
/// # Panics
///
/// When a fourth connection, three being open, is not answered 503; when
/// a GET is not served once one of the three has closed (tried until
/// [`PATIENCE`]); when the two left open are not closed when the server
/// stops; or when a connection cannot be made.
#[test]
fn connections_beyond_the_limit_are_turned_away() {
    let limits = Limits {
        request_time: Duration::from_secs(30),
        ..QUICK
    };
    let served = served("busy", limits, |address| {
        let mut held: Vec<TcpStream> = (0..limits.connections).map(|_| connect(address)).collect();
        // Answered busy before anything is read: asking nothing, the
        // connection is not reset for what it sent.
        let busy = rest(&mut connect(address));
        drop(held.pop());
        let started = Instant::now();
        let freed = loop {
            let mut stream = connect(address);
            let mut bytes = Vec::new();
            // The busy answer is written without reading the request, so
            // the connection may be reset under it: busy all the same.
            let read = stream
                .write_all(&get(address))
                .and_then(|()| stream.read_to_end(&mut bytes));
            let served = read.is_ok() && !bytes.starts_with(b"HTTP/1.1 503");
            if served || started.elapsed() > PATIENCE {
                break String::from_utf8(bytes).unwrap();
            }
            thread::sleep(Duration::from_millis(20));
        };
        (busy, freed, held)
    });
    let (busy, freed, held) = served.given;
    assert!(
        busy.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
        "{busy}"
    );
    assert!(busy.ends_with("md2html is busy; try again\n"), "{busy}");
    assert!(freed.starts_with("HTTP/1.1 200 OK\r\n"), "{freed}");
    for mut stream in held {
        assert_eq!(rest(&mut stream), "", "closed when the server stopped");
    }
}

/// The server stops with connections open, at once and whole.
///
/// A connection that asked nothing, one with half a request and a page's
/// reload signal each end, though their own limits are a minute and ten
/// minutes away, and every thread has ended when [`serve`] returns.
///
/// # Panics
///
/// When the reload signal does not start with render 1; when the server
/// takes five seconds or more to stop; when any of the three connections
/// does not end (within [`PATIENCE`]) or brings anything more; or when the
/// server keeps a trouble from its closing.
#[test]
fn the_server_stops_with_connections_open_and_leaves_nothing_behind() {
    let limits = Limits {
        connections: 64,
        request_time: Duration::from_secs(60),
        ..QUICK
    };
    let served = served("stop", limits, |address| {
        let silent = connect(address);
        let mut half = connect(address);
        half.write_all(b"GET /notes.html HTTP/1.1\r\n").unwrap();
        let mut signal = connect(address);
        let asked = format!(
            "GET /__md2html/events HTTP/1.1\r\nHost: localhost:{}\r\n\r\n",
            address.port()
        );
        signal.write_all(asked.as_bytes()).unwrap();
        let mut head = Vec::new();
        let mut byte = [0_u8];
        while !head.ends_with(b"data: 1\n\n") {
            signal.read_exact(&mut byte).unwrap();
            head.extend_from_slice(&byte);
        }
        [silent, half, signal]
    });
    assert!(
        served.stopping < Duration::from_secs(5),
        "stopped in {:?}",
        served.stopping
    );
    for mut stream in served.given {
        assert_eq!(rest(&mut stream), "", "the connection ended");
    }
    assert_eq!(served.troubles, Vec::<String>::new());
}

/// A run that panics while the server runs does not leave the server
/// waiting forever: the panic comes through, the server stopped.
///
/// # Panics
///
/// When the panic of the run does not come out of [`serve`], or only
/// after five seconds; when the listener cannot be opened or the site's
/// folder made or removed.
#[test]
fn a_panic_in_the_run_stops_the_server() {
    let socket = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let (site, dir) = site_within("panic", socket.local_addr().unwrap().port(), QUICK);
    let started = Instant::now();
    let ran = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        serve(&socket, &site, || panic!("the run failed"))
    }));
    assert!(ran.is_err(), "the panic came through");
    assert!(started.elapsed() < Duration::from_secs(5));
    std::fs::remove_dir_all(dir).unwrap();
}
