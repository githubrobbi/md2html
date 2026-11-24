// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One connection: the request read, the answer decided, the answer
//! written. Deciding is apart from the connection, so every rule the server
//! keeps can be tried with a request written by hand.

use core::time::Duration;
use std::ffi::OsStr;
use std::io::{BufReader, ErrorKind, Read as _, Write};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::Instant;

use super::request::{Deadline, Method, Read, Request, out_of_time, read_request};
use super::{Signal, Site};
use crate::system;
use crate::text::percent_decode_path;

/// How long the busy answer may wait for the browser: it is written on the
/// listener's thread, which a slow browser must not hold.
const BUSY_WRITE_TIME: Duration = Duration::from_secs(1);

/// How much of what the browser still sends after a refusal is read.
const LINGER_BYTES: u64 = 64 * 1024;

/// Where the reload signal is.
const EVENTS: &str = "/__md2html/events";

/// An answer's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    /// 200.
    Ok,
    /// 400: not a request, or not a whole one.
    BadRequest,
    /// 403: not addressed to this machine.
    Forbidden,
    /// 404: no such page or file, or one that is not served.
    NotFound,
    /// 405: neither GET nor HEAD.
    MethodNotAllowed,
    /// 408: the request did not arrive in its time.
    RequestTimeout,
    /// 414: the request line is longer than a line may be.
    UriTooLong,
    /// 431: a header is longer than a line may be, or there are too many.
    HeadersTooLarge,
    /// 503: every connection is taken.
    Busy,
    /// 505: an HTTP other than 1.
    VersionNotSupported,
}

impl Status {
    /// Its number and its reason.
    const fn line(self) -> (u16, &'static str) {
        match self {
            Self::Ok => (200, "OK"),
            Self::BadRequest => (400, "Bad Request"),
            Self::Forbidden => (403, "Forbidden"),
            Self::NotFound => (404, "Not Found"),
            Self::MethodNotAllowed => (405, "Method Not Allowed"),
            Self::RequestTimeout => (408, "Request Timeout"),
            Self::UriTooLong => (414, "URI Too Long"),
            Self::HeadersTooLarge => (431, "Request Header Fields Too Large"),
            Self::Busy => (503, "Service Unavailable"),
            Self::VersionNotSupported => (505, "HTTP Version Not Supported"),
        }
    }
}

/// Whether an answer's body goes with its head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Body {
    /// Sent: for GET, and for every refusal.
    Sent,
    /// Withheld: HEAD asks for the head alone.
    Withheld,
}

/// A whole answer.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Response {
    /// Its status.
    pub(super) status: Status,
    /// Its media type.
    pub(super) kind: &'static str,
    /// Its body.
    pub(super) body: Vec<u8>,
    /// Whether the body is sent.
    pub(super) send: Body,
}

impl Response {
    /// A short text answer.
    fn text(status: Status, text: &str, send: Body) -> Self {
        Self {
            status,
            kind: "text/plain",
            body: text.as_bytes().to_vec(),
            send,
        }
    }

    /// The answer to a request refused before it was read whole: its
    /// status and its reason.
    fn refusal(status: Status) -> Self {
        let (_, reason) = status.line();
        Self::text(status, &format!("{reason}\n"), Body::Sent)
    }
}

/// What the server does about a request.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// Answers it whole.
    Whole(Response),
    /// Holds the connection open for the reload signal.
    Events,
}

/// Serves one connection.
///
/// # Errors
///
/// The request broke off, or the answer could not be written.
pub(super) fn connection(stream: &TcpStream, site: &Site) -> std::io::Result<()> {
    let answered = answer(stream, site);
    // The listener's thread keeps a handle on the connection, to shut it
    // when the server closes; the browser is told the answer is over now.
    let shut = match stream.shutdown(Shutdown::Both) {
        Err(error) if !left_page(error.kind()) => Err(error),
        Ok(()) | Err(_) => Ok(()),
    };
    answered.and(shut)
}

/// Reads the request and answers it.
///
/// # Errors
///
/// As [`connection`].
fn answer(stream: &TcpStream, site: &Site) -> std::io::Result<()> {
    stream.set_write_timeout(Some(site.limits.write_time))?;
    let read = read_request(BufReader::new(Deadline::new(
        stream,
        site,
        site.limits.request_time,
    )))?;
    let mut out = stream;
    let response = match read {
        Read::Nothing => return Ok(()),
        Read::Refused(status) => Response::refusal(status),
        Read::Request(request) => match respond(&request, site) {
            Answer::Whole(response) => response,
            Answer::Events => return events(&mut out, site),
        },
    };
    write_response(&mut out, &response)?;
    if response.status == Status::Ok {
        Ok(())
    } else {
        linger(stream, site)
    }
}

/// Closes a connection after a refusal so that the browser reads it.
///
/// The answer is ended, and what the browser still sends (the rest of a
/// request too long, a body) is read and thrown away, for at most the
/// site's `linger` and [`LINGER_BYTES`]. Closed with it unread, the
/// connection would be reset, and the refusal lost with it.
///
/// # Errors
///
/// The connection failed in another way than the browser closing it.
fn linger(stream: &TcpStream, site: &Site) -> std::io::Result<()> {
    match stream.shutdown(Shutdown::Write) {
        Ok(()) => {}
        Err(error) if left_page(error.kind()) => return Ok(()),
        Err(error) => return Err(error),
    }
    let mut rest = Deadline::new(stream, site, site.limits.linger).take(LINGER_BYTES);
    match std::io::copy(&mut rest, &mut std::io::sink()) {
        Ok(_) => Ok(()),
        Err(error) if left_page(error.kind()) || out_of_time(error.kind()) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Tells a browser that every connection is taken.
///
/// # Errors
///
/// The answer could not be written.
pub(super) fn busy(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_write_timeout(Some(BUSY_WRITE_TIME))?;
    let mut out = stream;
    write_response(
        &mut out,
        &Response::text(Status::Busy, "md2html is busy; try again\n", Body::Sent),
    )
}

/// What to answer `request`.
pub(super) fn respond(request: &Request, site: &Site) -> Answer {
    let port = site.port;
    let ours = [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ];
    if !ours.contains(&request.host.to_ascii_lowercase()) {
        return Answer::Whole(Response::text(
            Status::Forbidden,
            "md2html serves this computer only\n",
            Body::Sent,
        ));
    }
    let send = match request.method {
        Method::Get => Body::Sent,
        Method::Head => Body::Withheld,
        Method::Other => {
            return Answer::Whole(Response::text(
                Status::MethodNotAllowed,
                "GET or HEAD\n",
                Body::Sent,
            ));
        }
    };
    // A path that decodes to bytes which are not UTF-8 names no file.
    let Some(path) = percent_decode_path(request.target.split(['?', '#']).next().unwrap_or("/"))
    else {
        return Answer::Whole(not_found(send));
    };
    if path == EVENTS {
        return Answer::Events;
    }
    if let Some(html) = site.page(&path) {
        return Answer::Whole(Response {
            status: Status::Ok,
            kind: "text/html; charset=utf-8",
            body: html.into_bytes(),
            send,
        });
    }
    Answer::Whole(file(&site.root, &path, send))
}

/// A file beside the pages: an image, a stylesheet. Nothing hidden,
/// nothing outside the folder served, links resolved.
///
/// What is asked for must name a file below the folder by names only
/// ([`system::file_below`]: read with the system's separators, not ending
/// in one, no empty part), none of them hidden (`.` included). Where it
/// leads, links followed, must be a file in the folder with no hidden name
/// on the way there: a visible link to a hidden file, or a name Windows
/// shortens (`GIT~1` for `.git`), is judged by what it resolves to.
fn file(root: &Path, path: &str, send: Body) -> Response {
    let rel = Path::new(path.strip_prefix('/').unwrap_or(path));
    let asked = system::file_below(rel).is_ok_and(|names| !names.into_iter().any(hidden));
    let wanted = root.join(rel);
    let inside = asked
        && std::fs::canonicalize(&wanted).is_ok_and(|real| {
            real.is_file()
                && real
                    .strip_prefix(root)
                    .is_ok_and(|under| !under.iter().any(hidden))
        });
    let read = if inside {
        std::fs::read(&wanted).ok()
    } else {
        None
    };
    read.map_or_else(
        || not_found(send),
        |body| Response {
            status: Status::Ok,
            kind: media_type(&wanted),
            body,
            send,
        },
    )
}

/// Whether a file's or a folder's name hides it: it starts with a dot.
fn hidden(name: &OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

/// The answer for a path that names nothing md2html serves.
fn not_found(send: Body) -> Response {
    Response::text(Status::NotFound, "not here\n", send)
}

/// Writes a whole answer.
///
/// # Errors
///
/// The browser did not take it.
fn write_response(out: &mut impl Write, response: &Response) -> std::io::Result<()> {
    let (code, reason) = response.status.line();
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
X-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        response.kind,
        response.body.len()
    );
    out.write_all(head.as_bytes())?;
    if response.send == Body::Sent {
        out.write_all(&response.body)?;
    }
    out.flush()
}

/// The reload signal, for the site's `stream_life`.
///
/// It says the render's number now, again whenever it changes, and a word
/// every `keep_alive` that it is still there. A page that is closed, or
/// the server closing, ends it.
///
/// # Errors
///
/// It could not be written for another reason than the page leaving.
pub(super) fn events(out: &mut impl Write, site: &Site) -> std::io::Result<()> {
    out.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n")?;
    let start = Instant::now();
    let mut seen = 0;
    while let Some(left) = site
        .limits
        .stream_life
        .checked_sub(start.elapsed())
        .filter(|left| !left.is_zero())
    {
        let sent = match site.next_signal(seen, left.min(site.limits.keep_alive)) {
            Signal::Same => out.write_all(b": still here\n\n"),
            Signal::Render(generation) => {
                seen = generation;
                out.write_all(format!("data: {generation}\n\n").as_bytes())
            }
            Signal::Closed => return Ok(()),
        };
        match sent.and_then(|()| out.flush()) {
            Ok(()) => {}
            Err(error) if left_page(error.kind()) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Whether a write failed because the page went away.
const fn left_page(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::BrokenPipe
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
    )
}

/// A served file's media type, from its name.
fn media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("woff2") => "font/woff2",
        Some("md" | "markdown" | "txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
