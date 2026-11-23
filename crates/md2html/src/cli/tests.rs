// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The command line and the files of a run, tried.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::args::{Action, Args, Command, Mode, Out, Shape, parse};
use super::files::{relative, unsafe_output, write_atomic};
use super::output::Exit;
use super::plan::{Dest, plan};

/// A fresh, empty folder for one test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("md2html-cli-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The command line `words` asks for.
fn command(words: &[&str]) -> Result<Command, String> {
    parse(words.iter().map(OsString::from))
}

/// The run `words` asks for.
fn run(words: &[&str]) -> Box<Args> {
    match command(words) {
        Ok(Command::Run(args)) => args,
        other => panic!("{words:?} is not a run: {other:?}"),
    }
}

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

#[test]
fn the_help_shows_a_neutral_example() {
    assert!(super::args::HELP.contains("brand: The Hillside house"));
}

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
    assert_eq!(run(&["a.md", "--ttl", "30"]).settings.live, Some(30));
}

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

#[cfg(unix)]
#[test]
fn an_input_that_is_not_unicode_is_still_an_input() {
    use std::os::unix::ffi::OsStringExt as _;
    let name = OsString::from_vec(b"caf\xe9.md".to_vec());
    let Ok(Command::Run(args)) = parse([name.clone()]) else {
        panic!("not a run");
    };
    assert_eq!(args.inputs, [PathBuf::from(name)]);
    let option = OsString::from_vec(b"--caf\xe9".to_vec());
    assert!(parse([option]).unwrap_err().ends_with("not valid Unicode"));
}

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
    std::fs::remove_dir_all(&dir).unwrap();
}

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

#[test]
fn exit_codes_keep_their_numbers() {
    let codes = [
        (Exit::Done, 0),
        (Exit::Findings, 1),
        (Exit::Usage, 64),
        (Exit::NoInput, 66),
        (Exit::Unavailable, 69),
        (Exit::Software, 70),
        (Exit::CantCreate, 73),
        (Exit::Io, 74),
    ];
    for (exit, code) in codes {
        assert_eq!(exit.code(), code, "{exit:?}");
    }
}

#[test]
fn pages_link_each_other_relatively_written_or_not() {
    let link = |from: &str, to: &str| relative(Path::new(from), Path::new(to));
    assert_eq!(
        link("never-site/sub/c.html", "never-site/z.html"),
        "../z.html"
    );
    assert_eq!(
        link("never-site/a.html", "never-site/sub/c.html"),
        "sub/c.html"
    );
    assert_eq!(link("./never-site/a.html", "never-site/b.html"), "b.html");
}
