// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The server's guarantees, tried with requests written by hand.

use alloc::sync::Arc;
use core::time::Duration;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use super::http::{Answer, Request, Response, Status, connection, events, read_request, respond};
use super::{MAX_CONNECTIONS, Pages, Site, Slot};

/// The port the test site says it listens on.
const PORT: u16 = 8123;

/// A folder with a page, a file beside it and a hidden one, served.
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
                with_body: true,
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
    assert!(!head.with_body, "HEAD has no body");
    std::fs::remove_dir_all(dir).unwrap();
}

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
