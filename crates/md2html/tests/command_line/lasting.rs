// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The runs that last or start another program: `--watch` rendering again
//! after a save, `--serve` answering at a page's link, and `--pdf` with the
//! Chrome it is given.

use core::time::Duration;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::process::Stdio;
use std::time::Instant;

use super::{Folder, NOTES};

/// How long a test waits for the watch to notice a save.
const PATIENCE: Duration = Duration::from_secs(20);

/// Waits until `ready` says yes, or [`PATIENCE`] is over.
fn wait_for(mut ready: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < PATIENCE {
        if ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    ready()
}

/// `--watch` renders the page at once, and again after each save.
///
/// # Panics
///
/// When the first page or the page after the save does not come within
/// [`PATIENCE`], the watch ends by itself, or it cannot be stopped.
#[test]
fn watch_renders_again_after_a_save() {
    let folder = Folder::new("watch");
    folder.write("notes.md", NOTES);
    let mut child = folder
        .md2html(&["--watch", "-q", "notes.md"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("md2html starts");
    let page = folder.path().join("notes.html");
    let shows = |words: &str| {
        wait_for(|| std::fs::read_to_string(&page).is_ok_and(|html| html.contains(words)))
    };
    let made = shows("A paragraph.");
    folder.write("notes.md", "# Notes\n\nSaved again, longer than before.\n");
    let again = made && shows("Saved again");
    let still_running = child.try_wait().expect("its state").is_none();
    child.kill().expect("the watch stopped");
    child.wait().expect("the watch ended");
    assert!(made, "the first page");
    assert!(again, "the page after the save");
    assert!(still_running, "the watch goes on while its input is there");
}

/// Serves the `inputs` from the folder with `--serve` on a port the
/// system gives, and asks it for `link`: the page the server names first,
/// and its answer.
///
/// # Panics
///
/// When md2html cannot be started or stopped, or does not say where it
/// serves, or does not answer.
fn serve_and_ask(folder: &Folder, inputs: &[&str], link: &str) -> (String, String) {
    let mut args = vec!["--serve", "--port", "0"];
    args.extend_from_slice(inputs);
    let mut child = folder
        .md2html(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("md2html starts");
    let stderr = child.stderr.take().expect("its standard error");
    let mut said = String::new();
    // The rest of what it says is read only after the request: its
    // standard error stays open until it is stopped.
    let mut lines = BufReader::new(stderr).lines();
    let served = lines.by_ref().map_while(Result::ok).find_map(|line| {
        said.push_str(&line);
        said.push('\n');
        line.split_once("serving http://127.0.0.1:")
            .map(|(_, rest)| rest.split(' ').next().unwrap_or_default().to_owned())
    });
    let asked = served.as_deref().and_then(|address| {
        let (port, first) = address.split_once('/')?;
        let mut stream = TcpStream::connect(("127.0.0.1", port.parse::<u16>().ok()?)).ok()?;
        stream.set_read_timeout(Some(PATIENCE)).ok()?;
        write!(
            stream,
            "GET /{link} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .ok()?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer).ok()?;
        Some((first.to_owned(), answer))
    });
    child.kill().expect("the server stopped");
    child.wait().expect("the server ended");
    drop(lines);
    asked.unwrap_or_else(|| panic!("no answer; md2html said:\n{said}"))
}

/// A page whose name a URL would read otherwise (`a b#c.md`: a space, and
/// a `#` that would end the path) is linked by `--nav` with its name
/// escaped, and `--serve` answers that link with the page.
///
/// # Panics
///
/// When the first page does not link the second by its name escaped, the
/// server does not say where it serves, by the same escaped name, or does
/// not answer a request for the link with the second page, or cannot be
/// stopped.
#[test]
fn a_page_linked_by_nav_is_served_at_its_link() {
    let folder = Folder::new("awkward-serve");
    folder.write("first.md", "# First\n\nThe first page.\n");
    folder.write("a b#c.md", "# Awkward\n\nThe page with the awkward name.\n");
    folder.run(&["first.md", "a b#c.md", "--nav"]).ends_with(0);
    let link = "a%20b%23c.html";
    assert!(
        folder
            .read("first.html")
            .contains(&format!("rel=\"next\" href=\"{link}\"")),
        "the link by --nav"
    );
    let (first, answer) = serve_and_ask(&folder, &["first.md", "a b#c.md"], link);
    assert_eq!(first, link, "the first page, as the server names it");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(
        answer.contains("The page with the awkward name."),
        "{answer}"
    );
}

/// `--serve` serves a handbook's chapters as a run writes them: the
/// chapters before and after in the start page's order, and the contents.
///
/// # Panics
///
/// When the server does not answer with the chapter, or the chapter lacks
/// its neighbours or the contents with itself marked.
#[test]
fn a_served_chapter_shows_its_handbook() {
    let folder = Folder::new("handbook-serve");
    folder.write(
        "manual/README.md",
        "# Manual\n\n[Second](second.md) [First](first.md)\n",
    );
    folder.write("manual/first.md", "# First\n\nA cluster.\n");
    folder.write("manual/second.md", "# Second\n");
    folder.write(
        "manual/glossary.md",
        "# Glossary\n\n**Cluster** - a unit.\n",
    );
    let (_, answer) = serve_and_ask(&folder, &["manual"], "first.html");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(
        answer.contains("rel=\"prev\" href=\"second.html\"")
            && answer.contains("rel=\"next\" href=\"glossary.html\""),
        "its neighbours in the start page's order"
    );
    assert!(
        answer.contains(
            "<li class=\"here\"><a href=\"first.html\" aria-current=\"page\">First</a></li>"
        ),
        "the contents"
    );
}

/// `--pdf` prints with the program `MD2HTML_CHROME` names, keeps what it
/// printed when it is a PDF, and ends with 70 when it is not.
///
/// # Panics
///
/// When the stand-in Chrome's PDF is not moved beside the page, a link
/// planted at the name of the hidden folder it prints into has the PDF
/// written through it, the run with a stand-in that prints nothing does
/// not end with 70 and say so, or a hidden file of the print stays behind.
#[cfg(unix)]
#[test]
fn a_pdf_is_what_chrome_printed() {
    use std::os::unix::fs::PermissionsExt as _;

    use super::Ran;

    let folder = Folder::new("pdf");
    folder.write("a.md", NOTES);
    let stand_in = |name: &str, body: &str| {
        let path = folder.write(name, format!("#!/bin/sh\n{body}\n"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("the stand-in may run");
        path
    };
    let chrome = stand_in(
        "chrome",
        "for a in \"$@\"; do case \"$a\" in --print-to-pdf=*) printf '%%PDF-1.7 stand-in' > \"${a#--print-to-pdf=}\";; esac; done",
    );
    let silent = stand_in("silent-chrome", "exit 0");
    let victim = folder.write("victim.txt", "mine");
    std::os::unix::fs::symlink(&victim, folder.path().join(".a.pdf.md2html-tmp"))
        .expect("a planted link");
    let with_pdf = Ran::of(
        folder
            .md2html(&["--pdf", "a.md"])
            .env("MD2HTML_CHROME", &chrome),
    );
    with_pdf.ends_with(0);
    assert_eq!(
        std::fs::read(folder.path().join("a.pdf")).expect("the PDF"),
        b"%PDF-1.7 stand-in"
    );
    assert!(
        with_pdf.out.contains("md2html: a.pdf\n"),
        "{}",
        with_pdf.out
    );
    assert_eq!(
        folder.read("victim.txt"),
        "mine",
        "nothing through the link"
    );
    let without = Ran::of(
        folder
            .md2html(&["--pdf", "-o", "b.html", "a.md"])
            .env("MD2HTML_CHROME", &silent),
    );
    without.ends_with(70);
    assert!(
        without.err.contains("Chrome printed no PDF"),
        "{}",
        without.err
    );
    let left: Vec<_> = std::fs::read_dir(folder.path())
        .expect("the folder")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|name| {
            name.to_str()
                .is_none_or(|text| text.contains("md2html-tmp"))
        })
        .collect();
    assert!(left.is_empty(), "{left:?}");
}
