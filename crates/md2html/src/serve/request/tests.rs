// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What is a request and what is not, read from bytes written by hand.

use super::super::http::Status;
use super::{Host, MAX_HEADERS, MAX_LINE, Method, Read, Request, read_request};

/// What reading `bytes` as a connection gives.
///
/// # Panics
///
/// When reading from memory fails, which it cannot.
fn read(bytes: &[u8]) -> Read {
    read_request(bytes).unwrap()
}

/// A request of `method` for `target`, addressed to `host`.
fn asked(method: Method, target: &str, host: &str) -> Read {
    Read::Request(Request {
        method,
        target: target.to_owned(),
        host: Host::Named(host.to_owned()),
    })
}

/// A request is read up to its headers and no further, in the forms a
/// browser sends it.
///
/// # Panics
///
/// When an empty connection reads as a request; when the method, the
/// target and a `Host` given in any case with spaces around it are not
/// read from a whole request; when lines ended by `\n` alone, one empty
/// line before the request, a method other than GET and HEAD, HTTP/1.0,
/// exactly [`MAX_HEADERS`] headers, a header value with bytes that are not
/// UTF-8, or a target written in full (whose authority stands for the
/// `Host`) are not read as the request they are.
#[test]
fn a_request_is_read_as_it_is_sent() {
    assert_eq!(read(b""), Read::Nothing, "a connection that asks nothing");
    assert_eq!(
        read(b"GET /a.html?x HTTP/1.1\r\nAccept: */*\r\nhost:  Localhost:8123 \r\n\r\nbody"),
        asked(Method::Get, "/a.html?x", "Localhost:8123")
    );
    assert_eq!(
        read(b"HEAD / HTTP/1.0\nHost: localhost:8123\n\n"),
        asked(Method::Head, "/", "localhost:8123"),
        "lines ended by a line feed alone"
    );
    assert_eq!(
        read(b"\r\nGET / HTTP/1.1\r\nHost: h\r\n\r\n"),
        asked(Method::Get, "/", "h"),
        "an empty line first"
    );
    assert_eq!(
        read(b"POST /form HTTP/1.1\r\nHost: h\r\nContent-Length: 3\r\n\r\nabc"),
        asked(Method::Other, "/form", "h"),
        "another method is read, and refused later"
    );
    let mut full = b"GET / HTTP/1.1\r\nHost: h\r\n".to_vec();
    for _ in 1..MAX_HEADERS {
        full.extend_from_slice(b"X-Filler: 1\r\n");
    }
    full.extend_from_slice(b"\r\n");
    assert_eq!(read(&full), asked(Method::Get, "/", "h"), "64 headers");
    assert_eq!(
        read(b"GET / HTTP/1.1\r\nHost: h\r\nX-Name: \xe9t\xe9\r\n\r\n"),
        asked(Method::Get, "/", "h"),
        "a value that is not UTF-8 is not the Host's business"
    );
    assert_eq!(
        read(b"GET http://LocalHost:8123/a.html?x HTTP/1.1\r\nHost: other\r\n\r\n"),
        asked(Method::Get, "/a.html?x", "LocalHost:8123"),
        "a target in full"
    );
    assert_eq!(
        read(b"GET http://localhost:8123?x HTTP/1.1\r\nHost: localhost:8123\r\n\r\n"),
        asked(Method::Get, "/?x", "localhost:8123"),
        "a target in full without a path"
    );
}

/// What is not a request is refused with 400 Bad Request, and an HTTP
/// other than 1 with 505.
///
/// # Panics
///
/// When any of the requests below (a line that is not `METHOD target
/// HTTP/1.x`, a target in neither form a server is sent, a header that is
/// not one, no `Host` or two, a `Host` that is not ASCII, a request that
/// ends before its empty line) is answered otherwise than 400; or when
/// HTTP/2.0 and HTTP/0.9 are not answered 505.
#[test]
fn a_malformed_request_is_answered_400() {
    let host = "Host: localhost:8123\r\n";
    for (line, why) in [
        ("GARBAGE", "one word"),
        ("GET", "no target"),
        ("GET /", "no version"),
        ("GET / HTTP/1.1 junk", "a word too many"),
        ("GET  / HTTP/1.1", "two spaces"),
        ("GET\t/ HTTP/1.1", "a tab for a space"),
        (" GET / HTTP/1.1", "a space first"),
        ("GET / FTP/1.1", "not HTTP"),
        ("GET / HTTP/1", "a version without its minor"),
        ("GET / HTTP/1.x", "a version that is not a number"),
        ("GET / HTTP/11.1", "a version of three digits"),
        ("G(T / HTTP/1.1", "a method that is not a token"),
        ("GET a.html HTTP/1.1", "a relative target"),
        ("GET * HTTP/1.1", "the asterisk"),
        ("CONNECT localhost:8123 HTTP/1.1", "an authority alone"),
        ("GET https://localhost:8123/ HTTP/1.1", "another scheme"),
        (
            "GET http:///a.html HTTP/1.1",
            "a full target without a name",
        ),
        ("GET /\u{e9}t\u{e9} HTTP/1.1", "a target that is not ASCII"),
    ] {
        let bytes = format!("{line}\r\n{host}\r\n");
        assert_eq!(
            read(bytes.as_bytes()),
            Read::Refused(Status::BadRequest),
            "{why}"
        );
    }
    for (bytes, why) in [
        (
            &b"GET /\x01 HTTP/1.1\r\nHost: h\r\n\r\n"[..],
            "a control character",
        ),
        (
            b"GET /\xff HTTP/1.1\r\nHost: h\r\n\r\n",
            "a byte that is not UTF-8",
        ),
        (
            b"GET / HTTP/1.1\rHost: h\r\n\r\n",
            "a carriage return alone",
        ),
        (b"GET / HTTP/1.1\r\n\r\n", "no Host"),
        (b"GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n", "two Hosts"),
        (
            b"GET / HTTP/1.1\r\nHost: \xc3\xa9t\xc3\xa9\r\n\r\n",
            "a Host that is not ASCII",
        ),
        (
            b"GET / HTTP/1.1\r\nHost : h\r\n\r\n",
            "a space before the colon",
        ),
        (
            b"GET / HTTP/1.1\r\nHost: h\r\n folded\r\n\r\n",
            "a folded line",
        ),
        (
            b"GET / HTTP/1.1\r\nHost: h\r\nNo colon\r\n\r\n",
            "a header without a colon",
        ),
        (
            b"GET / HTTP/1.1\r\nHost: h\r\n: value\r\n\r\n",
            "a header without a name",
        ),
        (
            b"GET / HTTP/1.1\r\nHost: h\r\nX: a\x00b\r\n\r\n",
            "a NUL in a value",
        ),
        (b"GET / HTTP/1.1\r\nHost: h\r\n", "no empty line"),
        (b"GET / HTTP/1.1\r\nHost: h", "a header cut short"),
        (b"GET / HTT", "a request line cut short"),
        (
            b"\r\n\r\nGET / HTTP/1.1\r\nHost: h\r\n\r\n",
            "two empty lines first",
        ),
    ] {
        assert_eq!(read(bytes), Read::Refused(Status::BadRequest), "{why}");
    }
    for version in ["HTTP/2.0", "HTTP/0.9"] {
        let bytes = format!("GET / {version}\r\n{host}\r\n");
        assert_eq!(
            read(bytes.as_bytes()),
            Read::Refused(Status::VersionNotSupported),
            "{version}"
        );
    }
}

/// A line over its limit is refused, not cut: 414 for the request line,
/// 431 for a header; more than [`MAX_HEADERS`] headers are 431 too.
///
/// # Panics
///
/// When a request line or a header line of [`MAX_LINE`] bytes with its
/// end is not read, when one a byte longer is not refused with 414 or 431,
/// or when a header past the 64th, even a `Host`, is read instead of 431.
#[test]
fn a_request_over_its_limits_is_refused_with_its_status() {
    let limit = usize::from(MAX_LINE);
    // `GET /…a HTTP/1.1\r\n`: 4 + target + 11 bytes.
    let at_limit = format!(
        "GET /{} HTTP/1.1\r\nHost: h\r\n\r\n",
        "a".repeat(limit - 16)
    );
    assert!(matches!(read(at_limit.as_bytes()), Read::Request(_)));
    let over = format!(
        "GET /{} HTTP/1.1\r\nHost: h\r\n\r\n",
        "a".repeat(limit - 15)
    );
    assert_eq!(read(over.as_bytes()), Read::Refused(Status::UriTooLong));
    // `X: …\r\n`: 3 + value + 2 bytes.
    let header = |value: usize| {
        format!(
            "GET / HTTP/1.1\r\nHost: h\r\nX: {}\r\n\r\n",
            "b".repeat(value)
        )
    };
    assert!(matches!(
        read(header(limit - 5).as_bytes()),
        Read::Request(_)
    ));
    assert_eq!(
        read(header(limit - 4).as_bytes()),
        Read::Refused(Status::HeadersTooLarge)
    );
    let mut flood = b"GET / HTTP/1.1\r\n".to_vec();
    for _ in 0..MAX_HEADERS {
        flood.extend_from_slice(b"X-Filler: 1\r\n");
    }
    flood.extend_from_slice(b"Host: localhost:8123\r\n\r\n");
    assert_eq!(read(&flood), Read::Refused(Status::HeadersTooLarge));
}

/// HTTP/1.0 lets a request leave `Host` out, and it is read without one;
/// HTTP/1.1 does not (RFC 9112, 3.2), and it is refused.
///
/// # Panics
///
/// When a GET or HEAD in HTTP/1.0 without `Host` is not read as a request
/// that names no host, or one with a full target not as addressed to its
/// authority; or when an HTTP/1.1 or HTTP/1.9 request without `Host` is
/// not answered 400.
#[test]
fn only_http_1_0_may_leave_host_out() {
    assert_eq!(
        read(b"GET /a.html HTTP/1.0\r\n\r\n"),
        Read::Request(Request {
            method: Method::Get,
            target: String::from("/a.html"),
            host: Host::Unnamed,
        })
    );
    assert_eq!(
        read(b"HEAD / HTTP/1.0\nAccept: */*\n\n"),
        Read::Request(Request {
            method: Method::Head,
            target: String::from("/"),
            host: Host::Unnamed,
        })
    );
    assert_eq!(
        read(b"GET http://localhost:8123/a.html HTTP/1.0\r\n\r\n"),
        asked(Method::Get, "/a.html", "localhost:8123")
    );
    for version in ["HTTP/1.1", "HTTP/1.9"] {
        let bytes = format!("GET / {version}\r\n\r\n");
        assert_eq!(
            read(bytes.as_bytes()),
            Read::Refused(Status::BadRequest),
            "{version}"
        );
    }
}
