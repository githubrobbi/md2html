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

use crate::system;

/// How long Chrome may take to print one page.
const PRINT_LIMIT: Duration = Duration::from_secs(90);

/// How often to look whether Chrome is done.
const POLL: Duration = Duration::from_millis(100);

/// Chrome, Chromium and Edge on macOS: in `/Applications`, or in
/// `Applications` in the home folder for a person's own install.
const MAC_APPS: [&str; 3] = [
    "Google Chrome.app/Contents/MacOS/Google Chrome",
    "Chromium.app/Contents/MacOS/Chromium",
    "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
];

/// Chrome, Chromium and Edge on Windows, under each of the program folders
/// (`ProgramFiles`, `ProgramFiles(x86)`, and `LOCALAPPDATA` for a person's
/// own install).
const WINDOWS_APPS: [&str; 3] = [
    r"Google\Chrome\Application\chrome.exe",
    r"Chromium\Application\chrome.exe",
    r"Microsoft\Edge\Application\msedge.exe",
];

/// The names they go by on `PATH`, before the system's suffix for a
/// program (`.exe` on Windows).
const ON_PATH: [&str; 7] = [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "chrome",
    "msedge",
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

/// Prints the page at `html` to `pdf` with Chrome.
///
/// The PDF has the page's own print layout, its headings as the PDF's
/// bookmarks, page numbers at the foot. Chrome runs headless, with a
/// throwaway profile, a time limit and no network: images on the web are
/// not fetched (`--embed` puts them inside). What it leaves behind and
/// cannot remove is named in the error, even when the PDF itself is
/// written.
///
/// # Errors
///
/// Returns an error when no Chrome is found ([`Places::chrome`]), when `html`
/// cannot be found, when `pdf` names no file, when Chrome cannot be started or
/// runs past [`PRINT_LIMIT`], and when
/// it leaves no PDF (`Chrome printed no PDF`), the folder it prints into
/// cannot be made, or the PDF cannot be moved to `pdf`. When Chrome's
/// profile or the hidden folder it printed into cannot be removed,
/// the error names each one left behind, after the reason the print failed or
/// after `the PDF is written, but`.
pub(crate) fn pdf(html: &Path, pdf: &Path) -> Result<(), String> {
    let chrome = Places::here().chrome(Path::is_file)?;
    let page = file_url(html)?;
    let scratch = Scratch::new(pdf)?;
    let printed = scratch
        .make()
        .and_then(|()| print(&chrome, &page, &scratch))
        .and_then(|()| scratch.keep(pdf));
    let leftovers = scratch.clear();
    if leftovers.is_empty() {
        return printed;
    }
    let left = leftovers.join("; ");
    Err(match printed {
        Ok(()) => format!("the PDF is written, but {left}"),
        Err(why) => format!("{why}; {left}"),
    })
}

/// Where `--pdf` looks for Chrome, as the environment says.
#[derive(Debug, Default)]
struct Places {
    /// `MD2HTML_CHROME`: the program to print with, when it is set.
    chosen: Option<PathBuf>,
    /// The home folder, for an install of a person's own on macOS.
    home: Option<PathBuf>,
    /// Windows' program folders: `ProgramFiles`, `ProgramFiles(x86)`,
    /// `LOCALAPPDATA`, those that are set.
    programs: Vec<PathBuf>,
    /// `PATH`.
    path: Option<OsString>,
}

impl Places {
    /// The places this run's environment names.
    fn here() -> Self {
        let programs = [
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
            std::env::var_os("LOCALAPPDATA"),
        ];
        Self {
            chosen: std::env::var_os("MD2HTML_CHROME").map(PathBuf::from),
            home: system::home(),
            programs: programs.into_iter().flatten().map(PathBuf::from).collect(),
            path: std::env::var_os("PATH"),
        }
    }

    /// Every file that may be Chrome, in the order they are tried: where
    /// macOS and Windows install it, then each folder of `PATH`.
    fn candidates(&self) -> Vec<PathBuf> {
        let applications = core::iter::once(PathBuf::from("/Applications"))
            .chain(self.home.iter().map(|home| home.join("Applications")));
        let mac = MAC_APPS
            .iter()
            .flat_map(|app| applications.clone().map(move |folder| folder.join(app)));
        let windows = self
            .programs
            .iter()
            .flat_map(|folder| WINDOWS_APPS.iter().map(move |app| folder.join(app)));
        let on_path = self
            .path
            .iter()
            .flat_map(std::env::split_paths)
            .flat_map(|folder| {
                ON_PATH
                    .iter()
                    .map(move |name| folder.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
            });
        mac.chain(windows).chain(on_path).collect()
    }

    /// The Chrome to print with: `MD2HTML_CHROME`, else the first of the
    /// [`candidates`](Self::candidates) that `is_file` finds.
    ///
    /// # Errors
    ///
    /// `MD2HTML_CHROME names <path>, which is not a file` when it is set to
    /// what is not a file (the other places are then not tried: the person
    /// chose), and `a PDF needs Chrome, Chromium or Edge, and none was found
    /// …` when no candidate is a file.
    fn chrome(&self, is_file: impl Fn(&Path) -> bool) -> Result<PathBuf, String> {
        if let Some(chosen) = &self.chosen {
            return if is_file(chosen) {
                Ok(chosen.clone())
            } else {
                Err(format!(
                    "MD2HTML_CHROME names {}, which is not a file",
                    chosen.display()
                ))
            };
        }
        self.candidates()
            .into_iter()
            .find(|path| is_file(path))
            .ok_or_else(|| {
                String::from(
                    "a PDF needs Chrome, Chromium or Edge, and none was found where they are installed or on PATH (MD2HTML_CHROME names one)",
                )
            })
    }
}

/// A file as a `file:` URL.
///
/// # Errors
///
/// Returns `finding <path>: <reason>` when `path` cannot be made absolute, most
/// often because there is no such file.
fn file_url(path: &Path) -> Result<String, String> {
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| format!("finding {}: {error}", path.display()))?;
    Ok(system::file_url(&absolute).into_string())
}

/// What a print keeps on disk while it runs: Chrome's throwaway profile,
/// and the PDF in a hidden folder beside it until it is whole.
///
/// Chrome opens the file it prints to by name and follows a link there, so
/// it prints into a folder md2html has just made, where nothing was
/// planted; the PDF is then renamed out of it, which replaces a link at
/// the PDF's own name rather than writing through it.
struct Scratch {
    /// Chrome's profile folder.
    profile: PathBuf,
    /// The hidden folder beside the PDF that Chrome prints into.
    folder: PathBuf,
    /// Where Chrome writes the PDF, in [`Scratch::folder`].
    printed: PathBuf,
}

impl Scratch {
    /// Names for one print of `pdf`, apart from any other run's.
    ///
    /// # Errors
    ///
    /// `<pdf> names no file` when `pdf` has no file name (`/`, `..`): the
    /// hidden file is named after it.
    fn new(pdf: &Path) -> Result<Self, String> {
        // A clock set before 1970 gives 0: the process id still keeps
        // two runs at once apart, and one run clears its profile before
        // the next print starts.
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let profile =
            std::env::temp_dir().join(format!("md2html-print-{}-{stamp}", std::process::id()));
        let Some(name) = pdf.file_name() else {
            return Err(format!("{} names no file", pdf.display()));
        };
        let mut hidden = OsString::from(".");
        hidden.push(name);
        hidden.push(".md2html-tmp");
        let folder = pdf.with_file_name(hidden);
        let printed = folder.join("printed.pdf");
        Ok(Self {
            profile,
            folder,
            printed,
        })
    }

    /// Makes the folder Chrome prints into, new: whatever is at its name
    /// (a folder a stopped run left, a file, a link planted there) is
    /// removed first, a link itself and not what it leads to.
    ///
    /// # Errors
    ///
    /// `making <folder>: <reason>` when what is there cannot be removed or
    /// the folder cannot be made.
    fn make(&self) -> Result<(), String> {
        let making = |error: std::io::Error| format!("making {}: {error}", self.folder.display());
        match std::fs::symlink_metadata(&self.folder) {
            Ok(found) if found.is_dir() => std::fs::remove_dir_all(&self.folder).map_err(making)?,
            Ok(_) => std::fs::remove_file(&self.folder).map_err(making)?,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(making(error)),
        }
        // New, or not at all: a folder or a link that appeared at the name
        // since is an error, never a place to print into.
        std::fs::DirBuilder::new()
            .recursive(false)
            .create(&self.folder)
            .map_err(making)
    }

    /// Gives what Chrome printed the PDF's name, if it is a PDF.
    ///
    /// # Errors
    ///
    /// Returns `Chrome printed no PDF` when the printed file is missing, is
    /// shorter than the PDF header, or does not start with `%PDF-`; `reading
    /// what Chrome printed, <file>: <reason>` when it cannot be read for
    /// another reason; and `writing <pdf>: <reason>` when it cannot be renamed
    /// to `pdf`.
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

    /// Removes the profile and the folder printed into, with the printed
    /// file unless it became the PDF; says what could not be removed.
    fn clear(&self) -> Vec<String> {
        [&self.profile, &self.folder]
            .into_iter()
            .filter_map(|folder| left_behind(folder, std::fs::remove_dir_all(folder)))
            .collect()
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
///
/// # Errors
///
/// Returns `starting <chrome>: <reason>` when the program cannot be started,
/// and the errors of [`wait`] while it runs.
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
///
/// # Errors
///
/// Returns `Chrome took longer than <n>s to print` when it runs past
/// [`PRINT_LIMIT`], and `watching Chrome: <reason>` when its state cannot be
/// read. Either way Chrome is stopped first, and the message adds what went
/// wrong when it could not be stopped or was not seen to end.
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
    use std::path::{Path, PathBuf};

    use super::{Places, Scratch, file_url, left_behind};

    /// Places with a home, one program folder and two folders on `PATH`,
    /// none of them real.
    ///
    /// # Panics
    ///
    /// When the two folders cannot be joined into a `PATH`.
    fn places() -> Places {
        Places {
            chosen: None,
            home: Some(PathBuf::from("/home/someone")),
            programs: vec![PathBuf::from("/programs")],
            path: Some(std::env::join_paths(["/bin-a", "/bin-b"]).unwrap()),
        }
    }

    /// Without a Chrome anywhere, `--pdf` says what it needs and where it
    /// looked, on every system.
    ///
    /// # Panics
    ///
    /// When nothing found gives a Chrome, or another message.
    #[test]
    fn without_chrome_a_pdf_says_what_it_needs() {
        for given in [Places::default(), places()] {
            assert_eq!(
                given.chrome(|_| false).unwrap_err(),
                "a PDF needs Chrome, Chromium or Edge, and none was found where they are installed or on PATH (MD2HTML_CHROME names one)"
            );
        }
    }

    /// `MD2HTML_CHROME` is the person's choice: one that is no file is said
    /// to be so, and no other Chrome is taken instead.
    ///
    /// # Panics
    ///
    /// When a chosen program that is there is not taken, or one that is not
    /// there falls back to another or gives another message.
    #[test]
    fn a_chosen_chrome_is_taken_or_named_as_missing() {
        let chosen = PathBuf::from("/opt/my-chrome");
        let given = Places {
            chosen: Some(chosen.clone()),
            ..places()
        };
        assert_eq!(given.chrome(|_| true).unwrap(), chosen);
        let only_others = |path: &Path| path != chosen;
        assert_eq!(
            given.chrome(only_others).unwrap_err(),
            format!(
                "MD2HTML_CHROME names {}, which is not a file",
                chosen.display()
            )
        );
    }

    /// The installed places are tried before `PATH`, a person's own macOS
    /// install after the system's, and a name on `PATH` carries the
    /// system's suffix for a program.
    ///
    /// # Panics
    ///
    /// When a candidate is missing, or the order is not macOS's places,
    /// then Windows', then `PATH`'s.
    #[test]
    fn installed_places_come_before_path() {
        let candidates = places().candidates();
        let at = |path: PathBuf| {
            candidates
                .iter()
                .position(|candidate| *candidate == path)
                .unwrap_or_else(|| panic!("{} is not tried: {candidates:?}", path.display()))
        };
        let system = at(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        let own = at(PathBuf::from("/home/someone/Applications")
            .join("Google Chrome.app/Contents/MacOS/Google Chrome"));
        let windows = at(PathBuf::from("/programs").join(r"Google\Chrome\Application\chrome.exe"));
        let exe = std::env::consts::EXE_SUFFIX;
        let first_on_path = at(PathBuf::from("/bin-a").join(format!("google-chrome{exe}")));
        let last_on_path = at(PathBuf::from("/bin-b").join(format!("msedge{exe}")));
        assert!(
            system < own
                && own < windows
                && windows < first_on_path
                && first_on_path < last_on_path
        );
        let found = places().chrome(|path| path.starts_with("/bin-b")).unwrap();
        assert_eq!(
            found,
            PathBuf::from("/bin-b").join(format!("google-chrome{exe}"))
        );
    }

    /// On Windows a program on `PATH` is found by its `.exe` name, and the
    /// per-user install under `LOCALAPPDATA` is tried.
    ///
    /// # Panics
    ///
    /// When `C:\tools\chrome.exe` on `PATH`, or Chrome under a user's
    /// `AppData\Local`, is not a candidate.
    #[cfg(windows)]
    #[test]
    fn on_windows_chrome_is_found_by_its_exe_name() {
        let given = Places {
            programs: vec![PathBuf::from(r"C:\Users\a\AppData\Local")],
            path: Some(std::ffi::OsString::from(r"C:\tools;C:\other")),
            ..Places::default()
        };
        let candidates = given.candidates();
        for wanted in [
            r"C:\tools\chrome.exe",
            r"C:\other\msedge.exe",
            r"C:\Users\a\AppData\Local\Google\Chrome\Application\chrome.exe",
        ] {
            assert!(
                candidates.contains(&PathBuf::from(wanted)),
                "{wanted}: {candidates:?}"
            );
        }
    }

    /// A path that is gone, or never made, is not reported as left behind.
    ///
    /// # Panics
    ///
    /// Panics when a removal that succeeded, or found nothing to remove, is
    /// reported, or when a removal that was refused is not reported as "nowhere
    /// is left behind: <reason>".
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

    /// Chrome prints into a hidden folder beside the PDF, with its profile in
    /// the temporary folder.
    ///
    /// # Panics
    ///
    /// Panics when `out/notes.pdf` is not printed into the folder
    /// `out/.notes.pdf.md2html-tmp`, when Chrome's profile is placed outside
    /// the system's temporary folder, or when a path without a file name is
    /// not refused as naming no file.
    #[test]
    fn the_pdf_is_printed_into_a_hidden_folder_beside_it() {
        let scratch = Scratch::new(Path::new("out/notes.pdf")).unwrap();
        assert_eq!(scratch.folder, Path::new("out/.notes.pdf.md2html-tmp"));
        assert_eq!(
            scratch.printed,
            Path::new("out/.notes.pdf.md2html-tmp/printed.pdf")
        );
        assert!(scratch.profile.starts_with(std::env::temp_dir()));
        for nameless in ["/", "out/.."] {
            let refused = Scratch::new(Path::new(nameless)).map(|_| ()).unwrap_err();
            assert_eq!(refused, format!("{nameless} names no file"));
        }
    }

    /// A page that does not exist gives no `file:` URL, and one that does gives
    /// an absolute one.
    ///
    /// # Panics
    ///
    /// Panics when a path to no file gives a URL, or when the crate's own
    /// folder does not give a URL that starts with `file:///`.
    #[test]
    fn a_missing_page_has_no_url() {
        file_url(Path::new("no/such/page.html")).unwrap_err();
        let url = file_url(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(url.starts_with("file:///"), "{url}");
    }

    /// The page Chrome is sent to print is named whatever its name holds:
    /// the name is one escaped segment of the `file:` URL.
    ///
    /// # Panics
    ///
    /// When the URL of a page with a name a URL would read otherwise does
    /// not end in that name escaped, or the scratch folder cannot be written
    /// in or removed.
    #[test]
    fn a_page_url_escapes_its_name() {
        let dir = std::env::temp_dir().join(format!("md2html-print-names-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for awkward in crate::tests::awkward_names() {
            let page = dir.join(format!("{}.html", awkward.name));
            std::fs::write(&page, "<p>x</p>").unwrap();
            let url = file_url(&page).unwrap();
            assert!(url.ends_with(&format!("/{}.html", awkward.url)), "{url}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
