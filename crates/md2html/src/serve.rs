// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html --serve`: the pages served on this computer while they are
//! written. Each save renders them again and every open page reloads,
//! keeping the reader's place (`page/live.js`). Nothing leaves the machine:
//! the server listens on the loopback address only, answers only to the
//! names of this machine (no DNS rebinding), serves only files inside the
//! document's folder, and only to GET and HEAD.

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

/// The pages and how many times they have been rendered.
struct State {
    /// URL path to page.
    pages: BTreeMap<String, String>,
    /// Bumped on every render; the pages' reload signal.
    generation: u64,
}

/// What the server shares between its threads.
type Shared = Arc<(Mutex<State>, Condvar)>;

/// Serves `render()`'s pages from `root` on 127.0.0.1, from `port` up,
/// rendering again when one of `watched` changes. Runs until stopped.
pub(crate) fn run(
    root: &Path,
    port: u16,
    watched: Vec<PathBuf>,
    render: impl Fn() -> BTreeMap<String, String> + Send + 'static,
    quiet: bool,
) -> i32 {
    let Some((listener, port)) = (port..port.saturating_add(20))
        .find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok().map(|l| (l, p)))
    else {
        eprintln!("md2html: no free port from {port} on 127.0.0.1");
        return 69;
    };
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let shared: Shared = Arc::new((
        Mutex::new(State {
            pages: render(),
            generation: 1,
        }),
        Condvar::new(),
    ));
    let first = shared
        .0
        .lock()
        .ok()
        .and_then(|s| s.pages.keys().next().cloned())
        .unwrap_or_default();
    if !quiet {
        eprintln!(
            "md2html: serving http://127.0.0.1:{port}{first} (this computer only); pages reload when the Markdown changes; Ctrl-C to stop"
        );
    }
    // The watcher: renders again when a file changes, and says so.
    let for_watcher = Arc::clone(&shared);
    std::thread::spawn(move || {
        let stamp = |paths: &[PathBuf]| -> Vec<Option<SystemTime>> {
            paths
                .iter()
                .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
                .collect()
        };
        let mut last = stamp(&watched);
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let now = stamp(&watched);
            if now != last {
                last = now;
                let pages = render();
                let (lock, signal) = &*for_watcher;
                if let Ok(mut state) = lock.lock() {
                    state.pages = pages;
                    state.generation += 1;
                }
                signal.notify_all();
            }
        }
    });
    for stream in listener.incoming().flatten() {
        let shared = Arc::clone(&shared);
        let root = root.clone();
        std::thread::spawn(move || {
            let _ = answer(stream, &shared, &root, port);
        });
    }
    0
}

/// Answers one request.
fn answer(mut stream: TcpStream, shared: &Shared, root: &Path, port: u16) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.by_ref().take(8192).read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, target) = (
        parts.next().unwrap_or_default().to_owned(),
        parts.next().unwrap_or_default().to_owned(),
    );
    // The headers: only Host matters; at most 64 of at most 8 KB.
    let mut host = String::new();
    for _ in 0..64 {
        let mut header = String::new();
        if reader.by_ref().take(8192).read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("host") {
                value.trim().clone_into(&mut host);
            }
        }
    }
    let ours = [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ];
    if !ours.contains(&host.to_ascii_lowercase()) {
        return reply(
            &mut stream,
            403,
            "text/plain",
            b"md2html serves this computer only\n",
            true,
        );
    }
    let head_only = method == "HEAD";
    if method != "GET" && !head_only {
        return reply(&mut stream, 405, "text/plain", b"GET or HEAD\n", true);
    }
    let path = crate::text::percent_decode_path(target.split(['?', '#']).next().unwrap_or("/"));
    if path == "/__md2html/events" {
        return events(stream, shared);
    }
    let page = shared.0.lock().ok().and_then(|state| {
        let key = if path == "/" {
            state
                .pages
                .keys()
                .find(|k| k.ends_with("/index.html"))
                .or_else(|| state.pages.keys().next())
                .cloned()
                .unwrap_or_default()
        } else {
            path.clone()
        };
        state.pages.get(&key).cloned()
    });
    if let Some(html) = page {
        return reply(
            &mut stream,
            200,
            "text/html; charset=utf-8",
            html.as_bytes(),
            !head_only,
        );
    }
    // A file beside the pages: an image, a stylesheet. Nothing hidden, nothing
    // outside.
    let rel = path.trim_start_matches('/');
    let hidden = rel
        .split('/')
        .any(|part| part.starts_with('.') || part.is_empty() && !rel.is_empty());
    let file = root.join(rel);
    let inside =
        std::fs::canonicalize(&file).is_ok_and(|real| real.starts_with(root) && real.is_file());
    if hidden || !inside {
        return reply(&mut stream, 404, "text/plain", b"not here\n", !head_only);
    }
    let bytes = std::fs::read(&file)?;
    reply(&mut stream, 200, media_type(&file), &bytes, !head_only)
}

/// The reload signal: the render's number now, and again whenever it changes.
fn events(mut stream: TcpStream, shared: &Shared) -> std::io::Result<()> {
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n")?;
    let (lock, signal) = &**shared;
    let mut seen = 0;
    loop {
        // The lock is held only while waiting; it is let go before the write.
        let generation = signal
            .wait_timeout_while(
                lock.lock().map_err(|_| std::io::Error::other("lock"))?,
                Duration::from_secs(15),
                |s| s.generation == seen,
            )
            .map_err(|_| std::io::Error::other("lock"))?
            .0
            .generation;
        if generation == seen {
            stream.write_all(b": still here\n\n")?;
        } else {
            seen = generation;
            stream.write_all(format!("data: {generation}\n\n").as_bytes())?;
        }
        stream.flush()?;
    }
}

/// Writes a whole response.
fn reply(
    stream: &mut TcpStream,
    status: u16,
    kind: &str,
    body: &[u8],
    with_body: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
X-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    if with_body {
        stream.write_all(body)?;
    }
    stream.flush()
}

/// A served file's media type, from its name.
fn media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
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
