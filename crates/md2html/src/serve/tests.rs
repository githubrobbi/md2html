// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The server's guarantees, tried with requests written by hand.

use core::time::Duration;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Instant;

use super::http::{Answer, Body, Response, Status, connection, events, respond};
use super::request::{Method, Request};
use super::{Limits, PORTS_TRIED, Pages, Signal, Site, first_free};
use crate::report::Exit;

mod tcp;

/// The port the test site says it listens on.
const PORT: u16 = 8123;

/// A folder with a page, a file beside it and a hidden one, served on
/// [`PORT`] within the server's own limits.
///
/// # Panics
///
/// As [`site_within`].
fn site(name: &str) -> (Site, PathBuf) {
    site_within(name, PORT, Limits::SERVE)
}

/// A folder with a page, a file beside it and a hidden one, served on
/// `port` within `limits`.
///
/// # Panics
///
/// When the scratch folder of an earlier run cannot be removed, or the
/// site's folder, its picture, its hidden file or the file outside it cannot
/// be written, or the folder cannot be resolved to its real path.
fn site_within(name: &str, port: u16, limits: Limits) -> (Site, PathBuf) {
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
    (Site::new(root, port, pages, limits), dir)
}

/// A request for `target` with `method`, addressed to `host`.
fn request(method: Method, target: &str, host: &str) -> Request {
    Request {
        method,
        target: target.to_owned(),
        host: host.to_owned(),
    }
}

/// The status `site` answers a GET of `target` with.
fn status(site: &Site, target: &str) -> Status {
    match respond(&request(Method::Get, target, "127.0.0.1:8123"), site) {
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
            respond(&request(Method::Get, "/notes.html", host), &site),
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
        let Answer::Whole(response) = respond(&request(Method::Get, "/notes.html", host), &site)
        else {
            panic!("{host}: the reload signal");
        };
        assert_eq!(response.status, Status::Forbidden, "{host}");
        let Answer::Whole(events) =
            respond(&request(Method::Get, "/__md2html/events", host), &site)
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
/// When another method is not refused with 405 Method Not Allowed; when
/// HEAD is not answered, or is answered with a body; or when the scratch
/// folder cannot be removed.
#[test]
fn only_get_and_head_are_answered() {
    let (site, dir) = site("method");
    let Answer::Whole(other) = respond(
        &request(Method::Other, "/notes.html", "localhost:8123"),
        &site,
    ) else {
        panic!("another method: the reload signal");
    };
    assert_eq!(other.status, Status::MethodNotAllowed);
    let Answer::Whole(head) = respond(
        &request(Method::Head, "/notes.html", "localhost:8123"),
        &site,
    ) else {
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
/// path, a full URL), a hidden file, a missing file, a file asked for as
/// a folder, or `/` on a site without pages is answered with anything but
/// 404; or when the scratch folder cannot be removed.
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
    let empty = Site::new(site.root, PORT, Pages::new(), Limits::SERVE);
    assert_eq!(
        status(&empty, "/"),
        Status::NotFound,
        "no page, no first page"
    );
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

/// A file's media type comes from its extension, in any case.
///
/// # Panics
///
/// When `Pic.PNG`, `notes.MD` or `style.Css` is not served with the type of
/// its kind, or a file with an unknown extension with another type than
/// `application/octet-stream`; or when the scratch folder cannot be written
/// in or removed.
#[test]
fn a_files_type_comes_from_its_extension_in_any_case() {
    let (site, dir) = site("types");
    for (name, kind) in [
        ("Pic.PNG", "image/png"),
        ("notes.MD", "text/plain; charset=utf-8"),
        ("style.Css", "text/css; charset=utf-8"),
        ("data.xyz", "application/octet-stream"),
    ] {
        std::fs::write(site.root.join(name), b"x").unwrap();
        let Answer::Whole(response) = respond(
            &request(Method::Get, &format!("/{name}"), "127.0.0.1:8123"),
            &site,
        ) else {
            panic!("{name}: the reload signal");
        };
        assert_eq!(response.kind, kind, "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// A symlink out of the served folder is not followed; one inside it is,
/// unless it leads to a hidden file.
///
/// # Panics
///
/// When a link to the file outside the folder, or a visible link to the
/// hidden file inside it, is served instead of 404, when a link to the
/// picture inside it is not served, or when a link or the scratch folder
/// cannot be made or removed.
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
    std::os::unix::fs::symlink(site.root.join(".secret"), site.root.join("shown.txt")).unwrap();
    assert_eq!(
        status(&site, "/shown.txt"),
        Status::NotFound,
        "a visible link to a hidden file"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// On Windows a backslash separates names in what is asked for, so a
/// hidden file behind one is not served, and a visible one is.
///
/// # Panics
///
/// When `sub\.secret` (written raw or as `%5C`) is served, `sub\shown.txt`
/// is not, or the scratch files cannot be made or removed.
#[cfg(windows)]
#[test]
fn on_windows_a_backslash_does_not_hide_a_hidden_file() {
    let (site, dir) = site("backslash");
    let sub = site.root.join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join(".secret"), b"hidden").unwrap();
    std::fs::write(sub.join("shown.txt"), b"shown").unwrap();
    for target in ["/sub%5C.secret", "/sub\\.secret", "/sub/.secret"] {
        assert_eq!(status(&site, target), Status::NotFound, "{target}");
    }
    for target in ["/sub%5Cshown.txt", "/sub\\shown.txt"] {
        assert_eq!(status(&site, target), Status::Ok, "{target}");
    }
    std::fs::remove_dir_all(dir).unwrap();
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
    let limits = Limits {
        stream_life: Duration::from_millis(30),
        ..Limits::SERVE
    };
    let (site, dir) = site_within("events", PORT, limits);
    let mut stream = Vec::new();
    events(&mut stream, &site).unwrap();
    let text = String::from_utf8(stream).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n"));
    assert!(
        text.contains("\r\n\r\ndata: 1\n\n: still here\n\n"),
        "{text}"
    );
    site.publish(Pages::new());
    assert_eq!(
        site.next_signal(1, Duration::ZERO),
        Signal::Render(2),
        "a render is counted"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// The reload signal ends at once when the server closes, however long it
/// had left.
///
/// # Panics
///
/// When a closed site's next signal is not [`Signal::Closed`], when a
/// reload signal with ten minutes to live does not end at once on a closed
/// site, or says anything after its head; or when the scratch folder
/// cannot be removed.
#[test]
fn the_reload_signal_ends_when_the_server_closes() {
    let (site, dir) = site("closing");
    site.close();
    assert_eq!(
        site.next_signal(0, Duration::from_secs(600)),
        Signal::Closed
    );
    let started = Instant::now();
    let mut stream = Vec::new();
    events(&mut stream, &site).unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "it ended at once"
    );
    assert!(
        String::from_utf8(stream)
            .unwrap()
            .ends_with("keep-alive\r\n\r\n"),
        "nothing after the head"
    );
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
    drop(client);
    server.join().unwrap();
    assert!(answer.starts_with("HTTP/1.1 403 Forbidden\r\n"), "{answer}");
    assert!(
        answer.ends_with("md2html serves this computer only\n"),
        "{answer}"
    );
    assert!(!answer.contains("notes"), "{answer}");
    std::fs::remove_dir_all(dir).unwrap();
}

/// The first port that opens is taken; when none does, the refusal says
/// why the last one was refused.
///
/// # Panics
///
/// When the third port is not taken after two in use, or when twenty ports
/// that may not be opened give another exit than `Unavailable` or a message
/// that does not name the last port and the reason.
#[test]
fn a_port_that_cannot_be_opened_is_named_with_its_reason() {
    let in_use = |tried: u16| {
        if tried < 9002 {
            Err(std::io::Error::from(std::io::ErrorKind::AddrInUse))
        } else {
            Ok(tried)
        }
    };
    assert_eq!(first_free(9000, in_use).unwrap(), (9002, 9002));
    let denied = |_: u16| -> std::io::Result<u16> {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    };
    let failure = first_free(9000, denied).unwrap_err();
    assert_eq!(failure.exit, Exit::Unavailable);
    let last = 9000 + PORTS_TRIED - 1;
    assert_eq!(
        failure.message,
        format!(
            "no free port from 9000 on 127.0.0.1 (port {last}: {})",
            std::io::Error::from(std::io::ErrorKind::PermissionDenied)
        )
    );
}
