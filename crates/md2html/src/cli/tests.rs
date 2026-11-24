// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The command line and the files of a run, tried.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::args::{Action, Args, Command, Mode, Out, megabytes, parse};
use super::files::{relative, unsafe_output, walk, write_atomic};
use super::plan::{Dest, plan};
use crate::config::Settings;
use crate::document::Shape;
use crate::report::Exit;
use crate::tests::not_unicode;

/// A fresh, empty folder for one test.
///
/// # Panics
///
/// When the folder of an earlier run cannot be removed, or the new one
/// cannot be created in the temporary directory.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("md2html-cli-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The command line `words` asks for.
///
/// # Errors
///
/// What [`parse`] refuses `words` for, in its words: an unknown option,
/// a missing value, no input.
fn command(words: &[&str]) -> Result<Command, String> {
    parse(words.iter().map(OsString::from))
}

/// The run `words` asks for.
///
/// # Panics
///
/// When `words` is not a run: [`parse`] refuses it, or reads it as
/// `--help` or `--version`.
fn run(words: &[&str]) -> Box<Args> {
    match command(words) {
        Ok(Command::Run(args)) => args,
        other => panic!("{words:?} is not a run: {other:?}"),
    }
}

/// `--help` and `--version` end the command line; the mistakes are named.
///
/// # Panics
///
/// When an option after `--help` or before `-V` is still read, or when an
/// unknown option, an empty command line, `-o` without a path or a `--port`
/// that is not a number gives no error or another message than the one a
/// person is told.
#[test]
fn help_and_version_end_the_command_line() {
    assert!(matches!(command(&["--help", "--bogus"]), Ok(Command::Help)));
    assert!(matches!(command(&["a.md", "-V"]), Ok(Command::Version)));
    assert_eq!(command(&["--bogus"]).unwrap_err(), "unknown option --bogus");
    assert_eq!(command(&[]).unwrap_err(), "no input");
    assert_eq!(command(&["-o"]).unwrap_err(), "-o needs a path");
    assert!(
        command(&["--port", "http"])
            .unwrap_err()
            .starts_with("--port http: a port number (")
    );
}

/// The help's front matter example names no real project.
///
/// # Panics
///
/// When the help no longer shows `brand: The Hillside house`.
#[test]
fn the_help_shows_a_neutral_example() {
    assert!(super::args::HELP.contains("brand: The Hillside house"));
}

/// The switches of a run set its action, shape, output and mode.
///
/// # Panics
///
/// When `--check`, `--fragment`, `-o -`, `-q` or `--nav` does not set what
/// it names; when `--watch` after `--serve` turns serving off, or `--port`
/// before `--serve` loses its port; when `--` does not make the next word
/// an input; or when `--max-input 2` is not 2 MB, or `--ttl 30` not 30
/// seconds of live.
#[test]
fn flags_that_are_modes_read_as_modes() {
    let args = run(&["a.md", "--check", "--fragment", "-o", "-", "-q", "--nav"]);
    assert_eq!(args.action, Action::Check);
    assert_eq!(args.shape, Shape::Fragment);
    assert_eq!(args.out, Out::Stdout);
    assert!(args.quiet && args.nav, "the two switches");
    assert_eq!(run(&["a.md", "--watch"]).mode, Mode::Watch);
    assert_eq!(run(&["a.md", "--serve", "--watch"]).mode, Mode::Serve(8000));
    assert_eq!(run(&["a.md", "--watch", "--serve"]).mode, Mode::Serve(8000));
    assert_eq!(
        run(&["a.md", "--port=9000", "--serve"]).mode,
        Mode::Serve(9000)
    );
    assert_eq!(run(&["--", "-dash.md"]).inputs, [PathBuf::from("-dash.md")]);
    assert_eq!(run(&["a.md", "--max-input", "2"]).max_input, 2 << 20_i32);
    assert_eq!(
        run(&["a.md", "--ttl", "30"]).settings.live,
        Some(crate::config::Ttl::seconds(30))
    );
}

/// `md2html in.md out.html` writes to the second file unless it is Markdown.
///
/// # Panics
///
/// When the second file of two is not read as the output, or when a second
/// `.md` that does not exist is taken as the output instead of refused.
#[test]
fn the_old_two_file_form_never_names_markdown_as_the_output() {
    let args = run(&["in.md", "out.html"]);
    assert_eq!(args.out, Out::Path(PathBuf::from("out.html")));
    assert_eq!(args.inputs, [PathBuf::from("in.md")]);
    let refused = command(&["in.md", "nowhere-such.md"]).unwrap_err();
    assert!(
        refused.starts_with("nowhere-such.md does not exist; the output would be a Markdown file")
    );
}

/// A file name that is not Unicode is an input; such an option is refused.
///
/// # Panics
///
/// When `name?.md` (`?` a unit no text holds) is not read as a run with that
/// one input, or the option `--name?` is not refused as not valid Unicode.
#[test]
fn an_input_that_is_not_unicode_is_still_an_input() {
    let name = not_unicode("name", ".md");
    let Ok(Command::Run(args)) = parse([name.clone()]) else {
        panic!("not a run");
    };
    assert_eq!(args.inputs, [PathBuf::from(name)]);
    let option = not_unicode("--name", "");
    assert!(parse([option]).unwrap_err().ends_with("not valid Unicode"));
}

/// A run never writes over its input or over another Markdown file.
///
/// # Panics
///
/// When the input given as its output, directly or by a roundabout path, is
/// not refused; when another `.md` or `.Markdown` file is not refused; when
/// an `.html` beside it is; or when the scratch folder cannot be made,
/// written in, or removed.
#[test]
fn the_input_is_never_the_output() {
    let dir = folder("same");
    let input = dir.join("notes.md");
    std::fs::write(&input, "# Notes\n").unwrap();
    assert_eq!(unsafe_output(&input, &input), Some("it is the input file"));
    let roundabout = dir.join(".").join("notes.md");
    assert_eq!(
        unsafe_output(&input, &roundabout),
        Some("it is the input file")
    );
    assert_eq!(
        unsafe_output(&input, &dir.join("other.md")),
        Some("the output would be a Markdown file")
    );
    assert_eq!(
        unsafe_output(&input, &dir.join("OTHER.Markdown")),
        Some("the output would be a Markdown file")
    );
    assert_eq!(unsafe_output(&input, &dir.join("notes.html")), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Where the file system ignores case (macOS, Windows), a name that
/// differs from the input's only in case is the input; where it does not
/// (Linux), it is another file.
///
/// # Panics
///
/// When `NOTES.TXT` is not refused as the input while it names the same
/// file as `notes.txt`, or is refused while it names none; or when the
/// scratch folder cannot be made, written in, or removed.
#[test]
fn a_name_in_another_case_is_the_input_where_case_is_ignored() {
    let dir = folder("case");
    let input = dir.join("notes.txt");
    std::fs::write(&input, "# Notes\n").unwrap();
    let other = dir.join("NOTES.TXT");
    let same_file = other.exists();
    assert_eq!(
        unsafe_output(&input, &other),
        same_file.then_some("it is the input file")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An output that is a symlink to the input is the input.
///
/// # Panics
///
/// When the output linked to the input is not refused as the input file, or
/// when the scratch folder, the input or the link cannot be made or removed.
#[cfg(unix)]
#[test]
fn an_output_that_links_to_the_input_is_the_input() {
    let dir = folder("same-link");
    let input = dir.join("notes.md");
    std::fs::write(&input, "# Notes\n").unwrap();
    let link = dir.join("page.html");
    std::os::unix::fs::symlink(&input, &link).unwrap();
    assert_eq!(unsafe_output(&input, &link), Some("it is the input file"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The plan of a run refuses unsafe outputs and finds every input.
///
/// # Panics
///
/// When writing over the input or a `.MD` file is not refused for a usage
/// error; when a folder of several pages for stdout is not refused, or is
/// refused under `--check`; when a single input's page does not go beside
/// it as `.html`; when a missing input is not refused as no input; or when
/// the scratch folder cannot be made, written in, or removed.
#[test]
fn a_run_refuses_to_write_over_its_input_or_a_markdown_file() {
    let dir = folder("plan");
    let input = dir.join("notes.md");
    std::fs::write(&input, "# Notes\n").unwrap();
    let shown = input.display().to_string();
    for out in [shown.clone(), dir.join("notes.MD").display().to_string()] {
        let failure = plan(&run(&[&shown, "-o", &out])).unwrap_err();
        assert_eq!(failure.exit, Exit::Usage, "{out}");
        assert!(
            failure.message.starts_with("refusing to write"),
            "{}",
            failure.message
        );
    }
    std::fs::write(dir.join("more.md"), "# More\n").unwrap();
    let folder_shown = dir.display().to_string();
    let several = plan(&run(&[&folder_shown, "-o", "-"])).unwrap_err();
    assert_eq!(several.exit, Exit::Usage);
    let jobs = plan(&run(&[&folder_shown, "--check", "-o", "-"])).unwrap();
    assert!(
        jobs.iter().all(|job| job.out == Dest::Stdout),
        "check writes nothing"
    );
    let beside = plan(&run(&[&shown])).unwrap();
    assert_eq!(
        beside.first().map(|job| &job.out),
        Some(&Dest::File(dir.join("notes.html")))
    );
    let missing = plan(&run(&[&dir.join("absent.md").display().to_string()])).unwrap_err();
    assert_eq!(missing.exit, Exit::NoInput);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A folder is walked index first, then by name in any case, files before
/// folders; hidden entries, `target` and `node_modules` are left out.
///
/// # Panics
///
/// When the Markdown files are found in another order, when a hidden file,
/// a file under `target` or `node_modules`, or a file that is not Markdown
/// is found, or when the scratch folder cannot be written in or removed.
#[test]
fn a_folder_is_walked_index_first_and_by_name() {
    let dir = folder("walk");
    for name in ["b.md", "A.md", "INDEX.md", ".hidden.md", "notes.txt"] {
        std::fs::write(dir.join(name), "# x\n").unwrap();
    }
    for sub in ["target", "node_modules", ".git", "sub"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
        std::fs::write(dir.join(sub).join("c.md"), "# c\n").unwrap();
    }
    let mut found = Vec::new();
    walk(&dir, &mut found).unwrap();
    assert_eq!(found, [
        Path::new("INDEX.md"),
        Path::new("A.md"),
        Path::new("b.md"),
        Path::new("sub/c.md")
    ]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A folder under an input that cannot be read stops the run before any
/// page is made, naming the folder; it is not taken for an empty one.
///
/// # Panics
///
/// When the plan of a folder with an unreadable sub-folder is made, or its
/// failure is not `NoInput` naming that sub-folder; when an unreadable input
/// folder is said to hold no Markdown; or when the scratch folder cannot be
/// made, locked, unlocked or removed. Run as root, where every folder can
/// be read, it checks nothing.
#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_stops_the_run() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = folder("locked");
    std::fs::write(dir.join("a.md"), "# a\n").unwrap();
    let locked = dir.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("b.md"), "# b\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&locked).is_err() {
        let failure = plan(&run(&[&dir.display().to_string(), "--check"])).unwrap_err();
        assert_eq!(failure.exit, Exit::NoInput);
        assert!(
            failure
                .message
                .starts_with(&format!("{} cannot be read: ", locked.display())),
            "{}",
            failure.message
        );
        let alone = plan(&run(&[&locked.display().to_string(), "--check"])).unwrap_err();
        assert!(
            alone.message.contains("cannot be read"),
            "{}",
            alone.message
        );
    }
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Names that are not Unicode are decided on their bytes.
///
/// Walked after every Unicode name, hidden when they start with `.`, an
/// index when they start with `index.`, and a page under such a name is
/// written whole under exactly that name. It needs a file system that takes
/// such names: Linux's (any bytes) and Windows' NTFS (a surrogate without
/// its pair); APFS refuses them.
///
/// # Panics
///
/// When the walk finds the files in another order or finds the hidden one,
/// when the page is not under its own name or its temporary file stays, or
/// when the scratch folder cannot be written in or removed.
#[cfg(any(target_os = "linux", windows))]
#[test]
fn names_that_are_not_unicode_are_taken_as_they_are() {
    use std::ffi::OsStr;
    let dir = folder("bytes");
    let odd = not_unicode("name", ".md");
    let first = not_unicode("INDEX.", ".md");
    for name in [OsStr::new("z.md"), &odd, &first, &not_unicode(".", ".md")] {
        std::fs::write(dir.join(name), "# x\n").unwrap();
    }
    let mut found = Vec::new();
    walk(&dir, &mut found).unwrap();
    let names: Vec<&OsStr> = found.iter().filter_map(|path| path.file_name()).collect();
    assert_eq!(names, [first.as_os_str(), OsStr::new("z.md"), &odd]);
    let page = dir.join(not_unicode("name", ".html"));
    write_atomic(&page, b"the page").unwrap();
    assert_eq!(std::fs::read(&page).unwrap(), b"the page");
    let left = dir.join(not_unicode(".name", ".html.md2html-tmp"));
    assert!(!left.exists(), "no temporary file is left");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `-o` ending in a separator names a folder, whatever bytes come before.
///
/// # Panics
///
/// When the page of `page.md` with `-o <dir>/name?/` (`?` a unit no text
/// holds) does not go to
/// `page.html` in that folder, or when the scratch folder cannot be made,
/// written in or removed.
#[test]
fn an_output_ending_in_a_separator_is_a_folder_whatever_its_bytes() {
    let dir = folder("slash");
    let input = dir.join("page.md");
    std::fs::write(&input, "# p\n").unwrap();
    let mut out = dir.join(not_unicode("name", "")).into_os_string();
    out.push("/");
    let Ok(Command::Run(args)) = parse([input.into_os_string(), OsString::from("-o"), out.clone()])
    else {
        panic!("not a run");
    };
    let jobs = plan(&args).unwrap();
    let wanted = PathBuf::from(out).join("page.html");
    assert_eq!(
        jobs.iter().map(|job| job.out.clone()).collect::<Vec<_>>(),
        [Dest::File(wanted)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A page is written whole, through a temporary file that does not stay.
///
/// # Panics
///
/// When the second or third write does not replace the page, when the
/// temporary file is left behind, when a stale temporary file stops the
/// write, when a write into a missing folder succeeds or gives no reason,
/// when a path without a file name is not refused as naming no file,
/// or when the scratch folder cannot be made or removed.
#[test]
fn a_file_is_written_whole_and_a_planted_link_is_not_followed() {
    let dir = folder("atomic");
    let page = dir.join("page.html");
    write_atomic(&page, b"first").unwrap();
    write_atomic(&page, b"second").unwrap();
    assert_eq!(std::fs::read(&page).unwrap(), b"second");
    let temporary = dir.join(".page.html.md2html-tmp");
    assert!(!temporary.exists(), "no temporary file is left");
    // A temporary file a stopped run left is replaced.
    std::fs::write(&temporary, b"stale").unwrap();
    write_atomic(&page, b"third").unwrap();
    assert_eq!(std::fs::read(&page).unwrap(), b"third");
    let failed = write_atomic(&dir.join("no-folder").join("page.html"), b"x").unwrap_err();
    assert!(!failed.is_empty(), "the reason is given");
    let nameless = dir.join("..");
    assert_eq!(
        write_atomic(&nameless, b"x").unwrap_err(),
        format!("{} names no file", nameless.display())
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A symlink planted at the temporary name is not written through.
///
/// # Panics
///
/// When the file the planted link points to is changed, when the page does
/// not hold what was written, or when the scratch folder, the file or the
/// link cannot be made or removed.
#[cfg(unix)]
#[test]
fn a_link_planted_at_the_temporary_name_is_not_followed() {
    let dir = folder("planted");
    let page = dir.join("page.html");
    let victim = dir.join("victim.txt");
    std::fs::write(&victim, b"keep me").unwrap();
    std::os::unix::fs::symlink(&victim, dir.join(".page.html.md2html-tmp")).unwrap();
    write_atomic(&page, b"the page").unwrap();
    assert_eq!(
        std::fs::read(&victim).unwrap(),
        b"keep me",
        "the link's target is untouched"
    );
    assert_eq!(std::fs::read(&page).unwrap(), b"the page");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An input is read only up to `--max-input`, whatever its size claims.
///
/// # Panics
///
/// When an input linked to `/dev/zero` is not refused as larger than 1 MB,
/// with the advice to raise `--max-input`; when a small file is not read
/// whole; or when the scratch folder, the link or the file cannot be made
/// or removed.
#[cfg(unix)]
#[test]
fn an_input_is_read_through_its_limit_whatever_it_claims() {
    use super::plan::Source;
    use super::run::read_source;

    let dir = folder("limit");
    let endless = dir.join("endless.md");
    std::os::unix::fs::symlink(Path::new("/dev/zero"), &endless).unwrap();
    let message = read_source(&Source::File(endless.clone()), 1 << 20).unwrap_err();
    assert_eq!(
        message,
        format!(
            "{}: larger than 1 MB (--max-input raises the limit)",
            endless.display()
        )
    );
    let small = dir.join("small.md");
    std::fs::write(&small, "# Small\n").unwrap();
    assert_eq!(
        read_source(&Source::File(small), 1 << 20).unwrap(),
        "# Small\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Links between pages are relative, whether the pages exist yet or not;
/// a page whose name is not Unicode gets no link.
///
/// # Panics
///
/// When the link from a page to one in a parent folder, in a sub-folder, or
/// beside it through a `./` path is not the shortest relative one, or when a
/// link to `name?.html` (`?` a unit no text holds) is made.
#[test]
fn pages_link_each_other_relatively_written_or_not() {
    let link = |from: &str, to: &str| relative(Path::new(from), Path::new(to));
    assert_eq!(
        link("never-site/sub/c.html", "never-site/z.html").as_deref(),
        Some("../z.html")
    );
    assert_eq!(
        link("never-site/a.html", "never-site/sub/c.html").as_deref(),
        Some("sub/c.html")
    );
    assert_eq!(
        link("./never-site/a.html", "never-site/b.html").as_deref(),
        Some("b.html")
    );
    let odd = PathBuf::from("never-site").join(not_unicode("name", ".html"));
    assert_eq!(relative(Path::new("never-site/a.html"), &odd), None);
}

/// On Linux and macOS a backslash is part of a name: it stays in a link,
/// and an output ending in one names a file.
///
/// # Panics
///
/// When the link to `x\y.html` is not that name, the page of `-o <dir>/out\`
/// does not go to that file, or the scratch folder cannot be made, written
/// in or removed.
#[cfg(unix)]
#[test]
fn on_unix_a_backslash_is_part_of_a_name() {
    assert_eq!(
        relative(
            Path::new("never-site/a.html"),
            Path::new("never-site/x\\y.html")
        )
        .as_deref(),
        Some("x\\y.html")
    );
    let dir = folder("backslash");
    let input = dir.join("page.md");
    std::fs::write(&input, "# p\n").unwrap();
    let out = dir.join("out\\");
    let Ok(Command::Run(args)) = parse([
        input.into_os_string(),
        OsString::from("-o"),
        out.clone().into_os_string(),
    ]) else {
        panic!("not a run");
    };
    let jobs = plan(&args).unwrap();
    assert_eq!(
        jobs.iter().map(|job| job.out.clone()).collect::<Vec<_>>(),
        [Dest::File(out)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// On Windows a backslash separates as `/` does: an output ending in one
/// is a folder and a link uses `/`; and pages on two drives link by their
/// `file:` URL, as no relative path joins them.
///
/// # Panics
///
/// When `-o <dir>\` does not put the page in that folder, the link to a
/// page in a sub-folder is not `sub/c.html`, or the link from drive `C:` to
/// drive `D:` is not `file:///D:/b/y.html`.
#[cfg(windows)]
#[test]
fn on_windows_a_backslash_separates_and_drives_link_by_url() {
    let dir = folder("backslash");
    let input = dir.join("page.md");
    std::fs::write(&input, "# p\n").unwrap();
    let mut out = dir.join("out").into_os_string();
    out.push("\\");
    let Ok(Command::Run(args)) = parse([input.into_os_string(), OsString::from("-o"), out.clone()])
    else {
        panic!("not a run");
    };
    let jobs = plan(&args).unwrap();
    assert_eq!(
        jobs.iter().map(|job| job.out.clone()).collect::<Vec<_>>(),
        [Dest::File(PathBuf::from(out).join("page.html"))]
    );
    assert_eq!(
        relative(
            Path::new(r"C:\site\a.html"),
            Path::new(r"C:\site\sub\c.html")
        )
        .as_deref(),
        Some("sub/c.html")
    );
    assert_eq!(
        relative(Path::new(r"C:\a\x.html"), Path::new(r"D:\b\y.html")).as_deref(),
        Some("file:///D:/b/y.html")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The size limits are lifted only on the command line.
///
/// # Panics
///
/// When front matter can set `max_embed`, when 50 megabytes are not read as
/// 50 MB, or when 0 or a word is taken as a size.
#[test]
fn limits_are_lifted_only_on_the_command_line() {
    let mut settings = Settings::default();
    assert!(
        settings.set("max_embed", "999").is_err(),
        "front matter cannot lift a limit"
    );
    assert_eq!(megabytes("50"), Ok(50 << 20_i32));
    assert!(megabytes("0").is_err() && megabytes("x").is_err());
}
