// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The server's guarantees, tried with requests written by hand.

use alloc::sync::Arc;
use core::time::Duration;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use super::http::{
    Answer, Body, Request, Response, Status, connection, events, read_request, respond,
};
use super::{MAX_CONNECTIONS, Pages, Site, Slot};

/// The port the test site says it listens on.
const PORT: u16 = 8123;

/// A folder with a page, a file beside it and a hidden one, served.
///
/// # Panics
///
/// When the scratch folder of an earlier run cannot be removed, or the
/// site's folder, its picture, its hidden file or the file outside it cannot
/// be written, or the folder cannot be resolved to its real path.
fn site(name: &str) -> (Site, PathBuf) {
    let dir = std::env::temp_dir().join(format!("md2html-serve-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    let folder = dir.join("site");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("picture.png"), b"\x89PNG").unwrap();
    std::fs::write(folder.join(".secret"), b"hidden").unwrap();
    std::fs::write(dir.join("outside.txt"), b"outside").unwrap();
    let root = std::fs::canonicalize(folder).unwrap();
    let mut pages = Pages::new();
    pages.insert(String::from("/notes.html"), String::from("<p>notes</p>"));
    (Site::new(root, PORT, pages), dir)
}

/// A request for `target` with `method`, addressed to `host`.
fn request(method: &str, target: &str, host: &str) -> Request {
    Request {
        method: method.to_owned(),
        target: target.to_owned(),
        host: host.to_owned(),
    }
}

/// The status `site` answers a GET of `target` with.
fn status(site: &Site, target: &str) -> Status {
    match respond(&request("GET", target, "127.0.0.1:8123"), site) {
        Answer::Whole(response) => response.status,
        Answer::Events => Status::Ok,
    }
}

/// Only a request whose `Host` names this machine and port is answered.
///
/// # Panics
///
/// When a request to `127.0.0.1`, `localhost` (in any case) or `[::1]` on the
/// site's port is not answered with the page; when one to another name,
/// another port, no port, no host or a name that only starts like
/// `127.0.0.1` gets anything but 403 Forbidden, for the page or for the
/// reload signal; or when the scratch folder cannot be removed.
#[test]
fn only_requests_addressed_to_this_machine_are_answered() {
    let (site, dir) = site("host");
    for host in [
        "127.0.0.1:8123",
        "localhost:8123",
        "LOCALHOST:8123",
        "[::1]:8123",
    ] {
        assert_eq!(
            respond(&request("GET", "/notes.html", host), &site),
            Answer::Whole(Response {
                status: Status::Ok,
                kind: "text/html; charset=utf-8",
                body: b"<p>notes</p>".to_vec(),
                send: Body::Sent,
            }),
            "{host}"
        );
    }
    for host in [
        "evil.example:8123",
        "127.0.0.1:9999",
        "localhost",
        "",
        "127.0.0.1.evil:8123",
    ] {
        let Answer::Whole(response) = respond(&request("GET", "/notes.html", host), &site) else {
            panic!("{host}: the reload signal");
        };
        assert_eq!(response.status, Status::Forbidden, "{host}");
        let Answer::Whole(events) = respond(&request("GET", "/__md2html/events", host), &site)
        else {
            panic!("{host}: the reload signal to a foreign host");
        };
        assert_eq!(events.status, Status::Forbidden, "{host}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Only GET and HEAD are answered; HEAD without a body.
///
/// # Panics
///
/// When POST, PUT, DELETE, OPTIONS, a lowercase `get` or an empty method is
/// not refused with 405 Method Not Allowed; when HEAD is not answered, or is
/// answered with a body; or when the scratch folder cannot be removed.
#[test]
fn only_get_and_head_are_answered() {
    let (site, dir) = site("method");
    for method in ["POST", "PUT", "DELETE", "OPTIONS", "get", ""] {
        let Answer::Whole(response) =
            respond(&request(method, "/notes.html", "localhost:8123"), &site)
        else {
            panic!("{method}");
        };
        assert_eq!(response.status, Status::MethodNotAllowed, "{method}");
    }
    let Answer::Whole(head) = respond(&request("HEAD", "/notes.html", "localhost:8123"), &site)
    else {
        panic!("HEAD");
    };
    assert_eq!(head.status, Status::Ok);
    assert_eq!(head.send, Body::Withheld, "HEAD has no body");
    std::fs::remove_dir_all(dir).unwrap();
}

/// No file outside the served folder, and no hidden file, is served.
///
/// # Panics
///
/// When the first page, a file with a query or a file named through a
/// percent escape is not served; when any way out of the folder (`..`
/// plain, escaped or behind `/site/`, `//`, an escaped `/`, an absolute
/// path, a full URL), a hidden file, a missing file or a file asked for as
/// a folder is answered with anything but 404; or when the scratch folder
/// cannot be removed.
#[test]
fn nothing_outside_the_folder_or_hidden_is_served() {
    let (site, dir) = site("paths");
    assert_eq!(status(&site, "/"), Status::Ok, "the first page");
    assert_eq!(status(&site, "/picture.png?v=1"), Status::Ok);
    assert_eq!(
        status(&site, "/pict%75re.png"),
        Status::Ok,
        "an escape inside the folder"
    );
    for target in [
        "/../outside.txt",
        "/%2e%2e/outside.txt",
        "/%2E%2E%2Foutside.txt",
        "/..%2foutside.txt",
        "/site/../../outside.txt",
        "//etc/passwd",
        "/%2fetc/passwd",
        "/etc/passwd",
        "/.secret",
        "/%2esecret",
        "/missing.png",
        "/picture.png/",
        "http://localhost:8123/../outside.txt",
    ] {
        assert_eq!(status(&site, target), Status::NotFound, "{target}");
    }
    let absolute = format!("/{}", dir.join("outside.txt").display());
    assert_eq!(status(&site, &absolute), Status::NotFound, "{absolute}");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A target whose escapes are not UTF-8 names no file.
///
/// # Panics
///
/// When the file named `\u{fffd}.png` is not served at its own escaped
/// name, when `%ff`, a lone `%C3` or `%80%80` is read as that file (or any
/// other) instead of 404, or when the scratch folder cannot be written in
/// or removed.
#[test]
fn a_path_that_is_not_utf8_names_no_file() {
    let (site, dir) = site("utf8");
    std::fs::write(site.root.join("\u{fffd}.png"), b"\x89PNG").unwrap();
    assert_eq!(
        status(&site, "/%EF%BF%BD.png"),
        Status::Ok,
        "the file's own name"
    );
    for target in ["/%ff.png", "/%C3.png", "/%80%80.png"] {
        assert_eq!(status(&site, target), Status::NotFound, "{target}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// A symlink out of the served folder is not followed; one inside it is.
///
/// # Panics
///
/// When a link to the file outside the folder is served instead of 404,
/// when a link to the picture inside it is not served, or when a link or
/// the scratch folder cannot be made or removed.
#[cfg(unix)]
#[test]
fn a_link_out_of_the_folder_is_not_followed() {
    let (site, dir) = site("links");
    std::os::unix::fs::symlink(dir.join("outside.txt"), site.root.join("link.txt")).unwrap();
    assert_eq!(
        status(&site, "/link.txt"),
        Status::NotFound,
        "a link out of the folder"
    );
    std::os::unix::fs::symlink(site.root.join("picture.png"), site.root.join("same.png")).unwrap();
    assert_eq!(
        status(&site, "/same.png"),
        Status::Ok,
        "a link inside the folder"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// A request is read up to its headers and no further.
///
/// # Panics
///
/// When an empty connection reads as a request; when the method, the
/// target and a `Host` given in any case with spaces around it are not
/// read from a whole request; when a `Host` after the first
/// [`super::http::MAX_HEADERS`] headers is still read; or when reading a
/// request from memory fails.
#[test]
fn a_request_is_read_within_its_limits() {
    let read = |bytes: &[u8]| read_request(bytes).unwrap();
    assert_eq!(read(b""), None, "a connection that asks nothing");
    assert_eq!(
        read(b"GET /a.html?x HTTP/1.1\r\nAccept: */*\r\nhost:  Localhost:8123 \r\n\r\nbody"),
        Some(request("GET", "/a.html?x", "Localhost:8123"))
    );
    // A Host after the 64th header is not read.
    let mut flood = b"GET / HTTP/1.1\r\n".to_vec();
    for _ in 0..super::http::MAX_HEADERS {
        flood.extend_from_slice(b"X-Filler: 1\r\n");
    }
    flood.extend_from_slice(b"Host: localhost:8123\r\n\r\n");
    assert_eq!(read(&flood), Some(request("GET", "/", "")));
}

/// The reload signal says the render's number, stays alive, and ends.
///
/// # Panics
///
/// When the signal does not start as a `text/event-stream`, does not send
/// render 1 and a keep-alive word within its 30 milliseconds, or is not
/// UTF-8 text; when publishing new pages does not count render 2; or when
/// the scratch folder cannot be removed.
#[test]
fn the_reload_signal_says_the_render_and_ends() {
    let (site, dir) = site("events");
    let mut stream = Vec::new();
    events(&mut stream, &site, Duration::from_millis(30)).unwrap();
    let text = String::from_utf8(stream).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n"));
    assert!(
        text.contains("\r\n\r\ndata: 1\n\n: still here\n\n"),
        "{text}"
    );
    site.publish(Pages::new());
    assert_eq!(
        site.generation_after(1, Duration::ZERO),
        2,
        "a render is counted"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// No more than [`MAX_CONNECTIONS`] connections are held at once.
///
/// # Panics
///
/// When more or fewer than [`MAX_CONNECTIONS`] places can be taken, when a
/// place given back cannot be taken again, or when the scratch folder
/// cannot be removed.
#[test]
fn connections_beyond_the_limit_are_turned_away() {
    let (served, dir) = site("slots");
    let site = Arc::new(served);
    let mut held = Vec::new();
    while let Some(slot) = Slot::take(&site) {
        held.push(slot);
    }
    assert_eq!(held.len(), MAX_CONNECTIONS, "every place is taken");
    held.clear();
    assert!(Slot::take(&site).is_some(), "places are given back");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A request for a foreign `Host` is refused over a real TCP connection.
///
/// # Panics
///
/// When the answer to a request for `attacker.example` is not 403 Forbidden
/// ending with `md2html serves this computer only`, or carries the page;
/// when the listener, the connection or the server's thread fails; or when
/// the scratch folder cannot be removed.
#[test]
fn a_foreign_host_is_refused_over_a_real_connection() {
    let (site, dir) = site("tcp");
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        connection(&stream, &site).unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET /notes.html HTTP/1.1\r\nHost: attacker.example:8123\r\n\r\n")
        .unwrap();
    let mut answer = String::new();
    client.read_to_string(&mut answer).unwrap();
    server.join().unwrap();
    assert!(answer.starts_with("HTTP/1.1 403 Forbidden\r\n"), "{answer}");
    assert!(
        answer.ends_with("md2html serves this computer only\n"),
        "{answer}"
    );
    assert!(!answer.contains("notes"), "{answer}");
    std::fs::remove_dir_all(dir).unwrap();
}
