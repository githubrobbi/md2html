// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A request read, or refused with the status that says why.
//!
//! Its first line and its headers are read, each line within its size and
//! the whole within its time, by the rules of RFC 9112. Nothing of a
//! request is kept but what the answer needs: the method, the target and
//! the name the server was reached by.

use core::time::Duration;
use std::io::{BufRead, ErrorKind, Read as _};
use std::net::TcpStream;
use std::time::Instant;

use super::http::Status;

#[cfg(test)]
mod tests;

/// The longest line of a request, its end included, in bytes.
pub(super) const MAX_LINE: u16 = 8192;

/// The most header lines read.
pub(super) const MAX_HEADERS: usize = 64;

/// A request's method: the server answers two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Method {
    /// `GET`.
    Get,
    /// `HEAD`: the head of the answer alone.
    Head,
    /// Any other: refused.
    Other,
}

/// What a browser asked.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Request {
    /// What it asks to be done.
    pub(super) method: Method,
    /// The path asked for, with its query.
    pub(super) target: String,
    /// The name the browser reached the server by: the `Host` header, or
    /// the authority of a target written in full (`http://host/path`).
    pub(super) host: String,
}

/// What a connection brought.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Read {
    /// Nothing: the browser opened the connection and asked nothing (a
    /// browser opens some ahead).
    Nothing,
    /// A request, whole.
    Request(Request),
    /// Something that is not a request, or not within the limits: refused
    /// with this status.
    Refused(Status),
}

/// Why a line stopped before its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    /// The browser closed the connection.
    Closed,
    /// The time to send the request ran out.
    OutOfTime,
}

impl Stop {
    /// The answer to a request that stopped this way after it began.
    const fn status(self) -> Status {
        match self {
            Self::Closed => Status::BadRequest,
            Self::OutOfTime => Status::RequestTimeout,
        }
    }
}

/// One line of a request, as read.
#[derive(Debug, PartialEq, Eq)]
enum Line {
    /// Whole, its end (`\r\n` or `\n`) taken off.
    Whole(Vec<u8>),
    /// Longer than [`MAX_LINE`].
    Cut,
    /// Begun, then stopped.
    Broken(Stop),
    /// Stopped before anything of it came.
    Silent(Stop),
}

/// Reads a request: one line, then header lines up to an empty one.
///
/// # Errors
///
/// The connection failed in another way than closing or running out of
/// time.
pub(super) fn read_request(mut reader: impl BufRead) -> std::io::Result<Read> {
    // One empty line before the request is passed over (RFC 9112, 2.2).
    let mut first = line(&mut reader)?;
    if first == Line::Whole(Vec::new()) {
        first = line(&mut reader)?;
    }
    let (method, target) = match first {
        Line::Whole(bytes) => match request_line(&bytes) {
            Ok(parts) => parts,
            Err(status) => return Ok(Read::Refused(status)),
        },
        Line::Cut => return Ok(Read::Refused(Status::UriTooLong)),
        Line::Broken(stop) => return Ok(Read::Refused(stop.status())),
        Line::Silent(_) => return Ok(Read::Nothing),
    };
    let named = match headers(&mut reader)? {
        Ok(named) => named,
        Err(status) => return Ok(Read::Refused(status)),
    };
    let (host, path) = match target {
        Target::Origin(path) => (named, path),
        Target::Absolute { authority, path } => (authority, path),
    };
    Ok(Read::Request(Request {
        method,
        target: path,
        host,
    }))
}

/// One line, read up to [`MAX_LINE`] bytes.
///
/// # Errors
///
/// Reading failed in another way than the browser closing the connection
/// or the time running out.
fn line(reader: &mut impl BufRead) -> std::io::Result<Line> {
    let mut bytes = Vec::new();
    let stop = match reader
        .by_ref()
        .take(u64::from(MAX_LINE))
        .read_until(b'\n', &mut bytes)
    {
        Ok(_) => Stop::Closed,
        Err(error) if out_of_time(error.kind()) => Stop::OutOfTime,
        Err(error) if closed(error.kind()) => Stop::Closed,
        Err(error) => return Err(error),
    };
    if let Some(body) = bytes.strip_suffix(b"\n") {
        let len = body.strip_suffix(b"\r").unwrap_or(body).len();
        bytes.truncate(len);
        return Ok(Line::Whole(bytes));
    }
    Ok(if bytes.len() >= usize::from(MAX_LINE) {
        Line::Cut
    } else if bytes.is_empty() {
        Line::Silent(stop)
    } else {
        Line::Broken(stop)
    })
}

/// Whether a read failed because its time ran out.
pub(super) const fn out_of_time(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

/// Whether a read failed because the browser let the connection go.
const fn closed(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::UnexpectedEof
    )
}

/// A request's target, in one of the two forms a server is sent.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// `/path?query`.
    Origin(String),
    /// `http://authority/path?query`: the authority stands for the `Host`.
    Absolute {
        /// The name and port, as written.
        authority: String,
        /// The path, `/` when none is written, with its query.
        path: String,
    },
}

/// The method and the target of a request line, `METHOD SP target SP
/// HTTP/1.x`.
///
/// # Errors
///
/// [`Status::BadRequest`] when the line is not of that shape (a space too
/// many or too few, a method that is not a token, a target with a space or
/// a control character, or in neither form a server is sent, text that is
/// not ASCII), [`Status::VersionNotSupported`] for an HTTP other than 1.
fn request_line(bytes: &[u8]) -> Result<(Method, Target), Status> {
    let text = core::str::from_utf8(bytes)
        .ok()
        .filter(|text| text.is_ascii())
        .ok_or(Status::BadRequest)?;
    let (verb, rest) = text.split_once(' ').ok_or(Status::BadRequest)?;
    let (target, version) = rest.split_once(' ').ok_or(Status::BadRequest)?;
    if verb.is_empty() || !verb.bytes().all(is_token) {
        return Err(Status::BadRequest);
    }
    if target.is_empty() || !target.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(Status::BadRequest);
    }
    let mut digits = version
        .strip_prefix("HTTP/")
        .ok_or(Status::BadRequest)?
        .bytes();
    match (digits.next(), digits.next(), digits.next(), digits.next()) {
        (Some(b'1'), Some(b'.'), Some(minor), None) if minor.is_ascii_digit() => {}
        (Some(major), Some(b'.'), Some(minor), None)
            if major.is_ascii_digit() && minor.is_ascii_digit() =>
        {
            return Err(Status::VersionNotSupported);
        }
        _ => return Err(Status::BadRequest),
    }
    let method = match verb {
        "GET" => Method::Get,
        "HEAD" => Method::Head,
        _ => Method::Other,
    };
    Ok((method, target_form(target).ok_or(Status::BadRequest)?))
}

/// The form `target` is written in; `None` when it is in neither form a
/// server is sent (`*`, `host:port`, a relative path, another scheme).
fn target_form(target: &str) -> Option<Target> {
    if target.starts_with('/') {
        return Some(Target::Origin(target.to_owned()));
    }
    let (scheme, rest) = target.split_at_checked("http://".len())?;
    if !scheme.eq_ignore_ascii_case("http://") {
        return None;
    }
    let (authority, after) = match rest.find(['/', '?']) {
        Some(at) => rest.split_at_checked(at)?,
        None => (rest, ""),
    };
    if authority.is_empty() {
        return None;
    }
    let path = if after.starts_with('/') {
        after.to_owned()
    } else {
        format!("/{after}")
    };
    Some(Target::Absolute {
        authority: authority.to_owned(),
        path,
    })
}

/// Reads the header lines up to the empty one; the `Host`.
///
/// # Errors
///
/// The outer error: as [`line`]. The inner: [`Status::HeadersTooLarge`]
/// for a line over [`MAX_LINE`] or more than [`MAX_HEADERS`] lines;
/// [`Status::BadRequest`] for a line that is not a header, no `Host` or
/// two, a `Host` that is not ASCII, or a request that ends before its
/// empty line; [`Status::RequestTimeout`] when the time runs out first.
fn headers(reader: &mut impl BufRead) -> std::io::Result<Result<String, Status>> {
    let mut host = None;
    let mut count = 0_usize;
    loop {
        let bytes = match line(reader)? {
            Line::Whole(bytes) => bytes,
            Line::Cut => return Ok(Err(Status::HeadersTooLarge)),
            Line::Broken(stop) | Line::Silent(stop) => return Ok(Err(stop.status())),
        };
        if bytes.is_empty() {
            return Ok(host.ok_or(Status::BadRequest));
        }
        count += 1;
        if count > MAX_HEADERS {
            return Ok(Err(Status::HeadersTooLarge));
        }
        let Some((name, value)) = field(&bytes) else {
            return Ok(Err(Status::BadRequest));
        };
        if name.eq_ignore_ascii_case(b"host") {
            let named = core::str::from_utf8(value)
                .ok()
                .filter(|text| text.is_ascii());
            match (named, &host) {
                (Some(text), None) => host = Some(text.to_owned()),
                _ => return Ok(Err(Status::BadRequest)),
            }
        }
    }
}

/// A header line's name and value, the value without spaces around it.
///
/// `None` when it is not a header: no colon, a name that is not a token
/// (a space before the colon, a line folded onto the one before), a
/// control character in the value.
fn field(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut parts = bytes.splitn(2, |&byte| byte == b':');
    let name = parts.next()?;
    let value = parts.next()?;
    let plain = |byte: &u8| *byte == b'\t' || (*byte >= b' ' && *byte != 0x7F);
    (!name.is_empty() && name.iter().copied().all(is_token) && value.iter().all(plain))
        .then(|| (name, value.trim_ascii()))
}

/// Whether `byte` may be part of a method or a header's name (a `tchar`,
/// RFC 9110, 5.6.2).
fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// A connection read until a deadline: however slowly a browser sends, it
/// holds the connection no longer.
pub(super) struct Deadline<'stream> {
    /// The connection.
    stream: &'stream TcpStream,
    /// How long it is read in all, for a deadline past what a clock holds.
    time: Duration,
    /// When reading ends; `None` for a deadline past what a clock holds.
    until: Option<Instant>,
}

impl<'stream> Deadline<'stream> {
    /// `stream`, read for `time` from now.
    pub(super) fn new(stream: &'stream TcpStream, time: Duration) -> Self {
        Self {
            stream,
            time,
            until: Instant::now().checked_add(time),
        }
    }
}

impl std::io::Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.map_or(self.time, |until| {
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
