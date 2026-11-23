// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The PDF: the page printed by the Chrome already on the machine.

use core::time::Duration;
use std::ffi::OsString;
use std::fs::File;
use std::io::{ErrorKind, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::text::put;

/// How long Chrome may take to print one page.
const PRINT_LIMIT: Duration = Duration::from_secs(90);

/// How often to look whether Chrome is done.
const POLL: Duration = Duration::from_millis(100);

/// Where Chrome, Chromium or Edge is usually installed.
const INSTALLED: [&str; 6] = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
];

/// The names it goes by on `PATH`.
const ON_PATH: [&str; 5] = [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
];

/// Chrome's switches for one quiet print: headless, nothing of its own
/// running beside it, the page's headings as the PDF's outline.
const SWITCHES: [&str; 16] = [
    "--headless",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--disable-background-networking",
    "--disable-sync",
    "--disable-component-update",
    "--no-pdf-header-footer",
    "--generate-pdf-document-outline",
    "--virtual-time-budget=5000",
    // No network while printing: names resolve nowhere and every
    // connection goes to a closed port, so a page cannot reach out.
    "--host-resolver-rules=MAP * ~NOTFOUND",
    "--proxy-server=socks5://127.0.0.1:9",
    "--proxy-bypass-list=<-loopback>",
    "--disable-domain-reliability",
    "--no-service-autorun",
];

/// What every PDF starts with.
const PDF_MAGIC: [u8; 5] = *b"%PDF-";

/// Prints the page at `html` to `pdf` with Chrome: the page's own print
/// layout, its headings as the PDF's bookmarks, page numbers at the foot.
/// Chrome runs headless, with a throwaway profile, a time limit and no
/// network: images on the web are not fetched (`--embed` puts them inside).
/// What it leaves behind and cannot remove is named in the error, even
/// when the PDF itself is written.
pub(crate) fn pdf(html: &Path, pdf: &Path) -> Result<(), String> {
    let chrome = chrome().ok_or_else(|| {
        String::from(
            "a PDF needs Chrome or Chromium, and none was found (set MD2HTML_CHROME to its program)",
        )
    })?;
    let page = file_url(html)?;
    let scratch = Scratch::new(pdf);
    let printed = print(&chrome, &page, &scratch).and_then(|()| scratch.keep(pdf));
    let leftovers = scratch.clear(printed.is_ok());
    if leftovers.is_empty() {
        return printed;
    }
    let left = leftovers.join("; ");
    Err(match printed {
        Ok(()) => format!("the PDF is written, but {left}"),
        Err(why) => format!("{why}; {left}"),
    })
}

/// The Chrome to print with: `MD2HTML_CHROME`, else the usual places.
fn chrome() -> Option<PathBuf> {
    if let Some(chosen) = std::env::var_os("MD2HTML_CHROME").map(PathBuf::from) {
        return chosen.is_file().then_some(chosen);
    }
    INSTALLED
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .or_else(on_path)
}

/// The first of [`ON_PATH`] found in a folder of `PATH`.
fn on_path() -> Option<PathBuf> {
    let folders = std::env::var_os("PATH")?;
    std::env::split_paths(&folders)
        .flat_map(|folder| ON_PATH.iter().map(move |name| folder.join(name)))
        .find(|path| path.is_file())
}

/// A file as a `file:` URL.
fn file_url(path: &Path) -> Result<String, String> {
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| format!("finding {}: {error}", path.display()))?;
    let text = absolute.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            out.push(char::from(byte));
        } else {
            put!(out, "%{byte:02X}");
        }
    }
    Ok(out)
}

/// What a print keeps on disk while it runs: Chrome's throwaway profile,
/// and the PDF under a hidden name until it is whole.
struct Scratch {
    /// Chrome's profile folder.
    profile: PathBuf,
    /// Where Chrome writes the PDF.
    printed: PathBuf,
}

impl Scratch {
    /// Names for one print of `pdf`, apart from any other run's.
    fn new(pdf: &Path) -> Self {
        // A clock set before 1970 gives 0: the process id still keeps
        // two runs at once apart, and one run clears its profile before
        // the next print starts.
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let profile =
            std::env::temp_dir().join(format!("md2html-print-{}-{stamp}", std::process::id()));
        // The PDF is named after the page, so it has a file name; were it
        // without one, the hidden file would be `..md2html-tmp` beside it.
        let mut hidden = OsString::from(".");
        hidden.push(pdf.file_name().unwrap_or_default());
        hidden.push(".md2html-tmp");
        Self {
            profile,
            printed: pdf.with_file_name(hidden),
        }
    }

    /// Gives what Chrome printed the PDF's name, if it is a PDF.
    fn keep(&self, pdf: &Path) -> Result<(), String> {
        let no_pdf = || String::from("Chrome printed no PDF");
        let mut head = [0_u8; PDF_MAGIC.len()];
        match File::open(&self.printed).and_then(|mut file| file.read_exact(&mut head)) {
            Ok(()) if head == PDF_MAGIC => {}
            Ok(()) => return Err(no_pdf()),
            Err(error)
                if error.kind() == ErrorKind::NotFound
                    || error.kind() == ErrorKind::UnexpectedEof =>
            {
                return Err(no_pdf());
            }
            Err(error) => {
                return Err(format!(
                    "reading what Chrome printed, {}: {error}",
                    self.printed.display()
                ));
            }
        }
        std::fs::rename(&self.printed, pdf)
            .map_err(|error| format!("writing {}: {error}", pdf.display()))
    }

    /// Removes the profile, and the printed file unless it became the
    /// PDF; says what could not be removed.
    fn clear(&self, kept: bool) -> Vec<String> {
        let mut left: Vec<String> =
            left_behind(&self.profile, std::fs::remove_dir_all(&self.profile))
                .into_iter()
                .collect();
        if !kept {
            left.extend(left_behind(
                &self.printed,
                std::fs::remove_file(&self.printed),
            ));
        }
        left
    }
}

/// What to say about removing `path`: nothing when it is gone, or was
/// never made (Chrome that does not start makes no profile).
fn left_behind(path: &Path, removal: std::io::Result<()>) -> Option<String> {
    match removal {
        Ok(()) => None,
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => Some(format!("{} is left behind: {error}", path.display())),
    }
}

/// Runs Chrome on `page` until it is done or out of time.
fn print(chrome: &Path, page: &str, scratch: &Scratch) -> Result<(), String> {
    let mut profile = OsString::from("--user-data-dir=");
    profile.push(&scratch.profile);
    let mut printed = OsString::from("--print-to-pdf=");
    printed.push(&scratch.printed);
    let mut child = Command::new(chrome)
        .args(SWITCHES)
        .arg(profile)
        .arg(printed)
        .arg(page)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("starting {}: {error}", chrome.display()))?;
    wait(&mut child)
}

/// Waits for Chrome to exit, and stops it when it takes longer than
/// [`PRINT_LIMIT`]. Its exit status is not read: whether a PDF came out
/// is what counts, and that is looked at next.
fn wait(child: &mut Child) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if started.elapsed() < PRINT_LIMIT => std::thread::sleep(POLL),
            Ok(None) => {
                let why = format!(
                    "Chrome took longer than {}s to print",
                    PRINT_LIMIT.as_secs()
                );
                return Err(stop(child, why));
            }
            Err(error) => return Err(stop(child, format!("watching Chrome: {error}"))),
        }
    }
}

/// Stops Chrome and waits for it to go; gives `why`, and what got in the
/// way if something did.
fn stop(child: &mut Child, why: String) -> String {
    match child.kill() {
        Ok(()) => match child.wait() {
            Ok(_) => why,
            Err(error) => format!("{why}, and it was stopped but not seen to end: {error}"),
        },
        // A child that has exited and been waited for: there is nothing
        // left to stop (the standard library has answered so, and says
        // it may again).
        Err(error) if error.kind() == ErrorKind::InvalidInput => why,
        Err(error) => format!("{why}, and it could not be stopped: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};
    use std::path::Path;

    use super::{Scratch, file_url, left_behind};

    #[test]
    fn what_is_gone_is_not_left_behind() {
        let path = Path::new("nowhere");
        assert_eq!(left_behind(path, Ok(())), None);
        assert_eq!(
            left_behind(path, Err(Error::from(ErrorKind::NotFound))),
            None
        );
        let left = left_behind(path, Err(Error::from(ErrorKind::PermissionDenied))).unwrap();
        assert!(left.starts_with("nowhere is left behind: "), "{left}");
    }

    #[test]
    fn the_pdf_is_printed_under_a_hidden_name_beside_it() {
        let scratch = Scratch::new(Path::new("out/notes.pdf"));
        assert_eq!(scratch.printed, Path::new("out/.notes.pdf.md2html-tmp"));
        assert!(scratch.profile.starts_with(std::env::temp_dir()));
    }

    #[test]
    fn a_missing_page_has_no_url() {
        file_url(Path::new("no/such/page.html")).unwrap_err();
        let url = file_url(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(url.starts_with("file:///"), "{url}");
    }
}
