// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One connection: the request read, the answer decided, the answer
//! written. Deciding is apart from the connection, so every rule the server
//! keeps can be tried with a request written by hand.

use core::time::Duration;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Instant;

use super::Site;
use crate::text::percent_decode_path;

/// How long a request has to arrive, whole.
pub(super) const HEAD_TIME: Duration = Duration::from_secs(10);

/// How long a reload signal lasts; the page then asks for a new one.
pub(super) const STREAM_LIFE: Duration = Duration::from_secs(600);

/// How long a write may wait for the browser to read.
const WRITE_TIME: Duration = Duration::from_secs(10);

/// How often a reload signal says it is still there.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// The longest line of a request read, in bytes.
const MAX_LINE: u64 = 8192;

/// The most header lines read.
pub(super) const MAX_HEADERS: usize = 64;

/// Where the reload signal is.
const EVENTS: &str = "/__md2html/events";

/// What a browser asked.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Request {
    /// `GET`, `HEAD`, ...
    pub(super) method: String,
    /// The path asked for, with its query.
    pub(super) target: String,
    /// The `Host` header: the name the browser reached the server by.
    pub(super) host: String,
}

/// An answer's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    /// 200.
    Ok,
    /// 403: not addressed to this machine.
    Forbidden,
    /// 404: no such page or file, or one that is not served.
    NotFound,
    /// 405: neither GET nor HEAD.
    MethodNotAllowed,
    /// 503: every connection is taken.
    Busy,
}

impl Status {
    /// Its number and its reason.
    const fn line(self) -> (u16, &'static str) {
        match self {
            Self::Ok => (200, "OK"),
            Self::Forbidden => (403, "Forbidden"),
            Self::NotFound => (404, "Not Found"),
            Self::MethodNotAllowed => (405, "Method Not Allowed"),
            Self::Busy => (503, "Service Unavailable"),
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
    stream.set_write_timeout(Some(WRITE_TIME))?;
    let deadline = Deadline {
        stream,
        until: Instant::now().checked_add(HEAD_TIME),
    };
    let Some(request) = read_request(BufReader::new(deadline))? else {
        return Ok(());
    };
    let mut out = stream;
    match respond(&request, site) {
        Answer::Whole(response) => write_response(&mut out, &response),
        Answer::Events => events(&mut out, site, STREAM_LIFE),
    }
}

/// Tells a browser that every connection is taken.
///
/// # Errors
///
/// The answer could not be written.
pub(super) fn busy(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut out = stream;
    write_response(
        &mut out,
        &Response::text(Status::Busy, "md2html is busy; try again\n", Body::Sent),
    )
}

/// A connection read until a deadline: however slowly a browser sends, it
/// holds the connection no longer.
struct Deadline<'stream> {
    /// The connection.
    stream: &'stream TcpStream,
    /// When reading ends; `None` for a deadline past what a clock holds.
    until: Option<Instant>,
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.map_or(HEAD_TIME, |until| {
            until.saturating_duration_since(Instant::now())
        });
        if left.is_zero() {
            return Err(std::io::Error::from(ErrorKind::TimedOut));
        }
        self.stream.set_read_timeout(Some(left))?;
        let mut stream = self.stream;
        stream.read(buf)
    }
}

/// Reads a request's first line and its `Host`; `None` when the browser
/// opened the connection and asked nothing (a browser opens some ahead).
///
/// # Errors
///
/// The request broke off after it began.
pub(super) fn read_request(mut reader: impl BufRead) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    match reader.by_ref().take(MAX_LINE).read_line(&mut line) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(error) if line.is_empty() && quiet_end(error.kind()) => return Ok(None),
        Err(error) => return Err(error),
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    // The headers: only Host matters; at most 64 of at most 8 KB.
    let mut host = String::new();
    for _ in 0..MAX_HEADERS {
        let mut header = String::new();
        if reader.by_ref().take(MAX_LINE).read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("host") {
                value.trim().clone_into(&mut host);
            }
        }
    }
    Ok(Some(Request {
        method,
        target,
        host,
    }))
}

/// Whether a read that failed before anything arrived means the browser
/// let the connection go.
const fn quiet_end(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::WouldBlock
            | ErrorKind::TimedOut
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::UnexpectedEof
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
    let send = match request.method.as_str() {
        "GET" => Body::Sent,
        "HEAD" => Body::Withheld,
        _ => {
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
fn file(root: &Path, path: &str, send: Body) -> Response {
    let rel = path.trim_start_matches('/');
    let hidden = rel
        .split('/')
        .any(|part| part.starts_with('.') || part.is_empty() && !rel.is_empty());
    let wanted = root.join(rel);
    let inside =
        std::fs::canonicalize(&wanted).is_ok_and(|real| real.starts_with(root) && real.is_file());
    let read = if hidden || !inside {
        None
    } else {
        std::fs::read(&wanted).ok()
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

/// The reload signal for `life`: the render's number now, again whenever
/// it changes, and a word every [`KEEP_ALIVE`] that it is still there. A
/// page that is closed ends it.
///
/// # Errors
///
/// It could not be written for another reason than the page leaving.
pub(super) fn events(out: &mut impl Write, site: &Site, life: Duration) -> std::io::Result<()> {
    out.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n")?;
    let start = Instant::now();
    let mut seen = 0;
    while let Some(left) = life
        .checked_sub(start.elapsed())
        .filter(|left| !left.is_zero())
    {
        let generation = site.generation_after(seen, left.min(KEEP_ALIVE));
        let sent = if generation == seen {
            out.write_all(b": still here\n\n")
        } else {
            seen = generation;
            out.write_all(format!("data: {generation}\n\n").as_bytes())
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
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
    )
}

/// A served file's media type, from its name.
fn media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
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
